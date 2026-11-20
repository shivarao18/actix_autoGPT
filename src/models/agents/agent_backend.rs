use crate::ai_functions::aifunc_backend::{
    print_backend_webserver_code, print_fixed_code, print_improved_webserver_code,
    print_rest_api_endpoints,
};
use crate::helpers::command_line::{PrintCommand, confirm_safe_code};
use crate::helpers::general::{
    ai_task_request, read_code_template_contents, read_exec_main_contents, save_api_endpoints,
    save_backend_code,
};
use crate::models::agent_basic::basic_agent::{AgentState, BasicAgent};
use crate::models::agents::agent_traits::{AgentError, FactSheet, RouteObject, SpecialFunctions};
use crate::run_context;
use crate::trajectory;
use crate::verification::{self, VerificationReport, static_get_routes};

use async_trait::async_trait;
use std::time::Instant;
use tracing::Instrument;

#[derive(Debug)]
pub struct AgentBackendDeveloper {
    attributes: BasicAgent,
    bug_errors: Option<String>,
    bug_count: u8,
}

impl AgentBackendDeveloper {
    pub fn new() -> Self {
        let attributes = BasicAgent {
            objective: "Develops backend code for webserver and json database".to_string(),
            position: "Backend Developer".to_string(),
            state: AgentState::Discovery,
            memory: vec![],
        };
        Self {
            attributes,
            bug_errors: None,
            bug_count: 0,
        }
    }

    async fn call_initial_backend_code(
        &mut self,
        factsheet: &mut FactSheet,
    ) -> Result<(), AgentError> {
        let code_template_str = read_code_template_contents()?;
        let msg_context = format!(
            "CODE TEMPLATE: {code_template_str} \n PROJECT_DESCRIPTION: {} \n",
            factsheet.project_description
        );
        let ai_response = ai_task_request(
            msg_context,
            &self.attributes.position,
            get_function_string!(print_backend_webserver_code),
            print_backend_webserver_code,
        )
        .await?;
        save_backend_code(&ai_response)?;
        factsheet.backend_code = Some(ai_response);
        Ok(())
    }

    async fn call_improved_backend_code(
        &mut self,
        factsheet: &mut FactSheet,
    ) -> Result<(), AgentError> {
        let msg_context = format!(
            "CODE TEMPLATE: {:?} \n PROJECT_DESCRIPTION: {:?} \n",
            factsheet.backend_code, factsheet
        );
        let ai_response = ai_task_request(
            msg_context,
            &self.attributes.position,
            get_function_string!(print_improved_webserver_code),
            print_improved_webserver_code,
        )
        .await?;
        save_backend_code(&ai_response)?;
        factsheet.backend_code = Some(ai_response);
        Ok(())
    }

    async fn call_fix_code_bugs(&mut self, factsheet: &mut FactSheet) -> Result<(), AgentError> {
        let msg_context = format!(
            "BROKEN_CODE: {:?} \n ERROR_BUGS: {:?} \n
      THIS FUNCTION ONLY OUTPUTS CODE. JUST OUTPUT THE CODE.",
            factsheet.backend_code, self.bug_errors
        );
        let ai_response = ai_task_request(
            msg_context,
            &self.attributes.position,
            get_function_string!(print_fixed_code),
            print_fixed_code,
        )
        .await?;
        save_backend_code(&ai_response)?;
        factsheet.backend_code = Some(ai_response);
        Ok(())
    }

    async fn call_extract_rest_api_endpoints(&self) -> Result<String, AgentError> {
        let backend_code = read_exec_main_contents()?;
        let msg_context = format!("CODE_INPUT: {backend_code}");
        ai_task_request(
            msg_context,
            &self.attributes.position,
            get_function_string!(print_rest_api_endpoints),
            print_rest_api_endpoints,
        )
        .await
    }

    async fn unit_test(&mut self, factsheet: &mut FactSheet) -> Result<(), AgentError> {
        PrintCommand::UnitTest.print_agent_message(
            self.attributes.position.as_str(),
            "Backend Code Unit Testing: Requesting user input",
        );
        if !confirm_safe_code() {
            return Err("User rejected generated code before it was built".into());
        }

        PrintCommand::UnitTest.print_agent_message(
            self.attributes.position.as_str(),
            "Backend Code Unit Testing: building project...",
        );

        let project_dir = crate::helpers::general::web_server_project_path();
        let started = Instant::now();
        let build = verification::build_project(&project_dir).await?;
        trajectory::record_tool(
            &self.attributes.position,
            "cargo_build",
            project_dir.to_string_lossy().as_ref(),
            &format!("{}\n{}", build.stdout, build.stderr),
            build.success,
            if build.success {
                None
            } else {
                Some(crate::llm::excerpt(&build.stderr, 1500))
            },
            started.elapsed().as_millis(),
        );

        if !build.success {
            self.bug_count += 1;
            self.bug_errors = Some(build.stderr.clone());
            trajectory::set_verification(VerificationReport {
                build: build.clone(),
                smoke: None,
                endpoints: Vec::new(),
            });
            if self.bug_count > 2 {
                PrintCommand::Issue.print_agent_message(
                    self.attributes.position.as_str(),
                    "Backend Code Unit Testing: Too many bugs found in code",
                );
                return Err("Error: Too many bugs".into());
            }
            self.attributes.state = AgentState::Working;
            return Ok(());
        }

        self.bug_count = 0;
        PrintCommand::UnitTest.print_agent_message(
            self.attributes.position.as_str(),
            "Backend Code Unit Testing: Test server build successful...",
        );

        let api_endpoints_str = self.call_extract_rest_api_endpoints().await?;
        let api_endpoints: Vec<RouteObject> = match serde_json::from_str(api_endpoints_str.as_str())
        {
            Ok(endpoints) => endpoints,
            Err(err) => {
                let message =
                    format!("Failed to decode API Endpoints: {err}; body: {api_endpoints_str}");
                trajectory::mark_last_step_failed(&message);
                return Err(message.into());
            }
        };

        let check_endpoints: Vec<RouteObject> = api_endpoints
            .iter()
            .filter(|route_object| {
                static_get_routes(&route_object.method, &route_object.is_route_dynamic)
            })
            .cloned()
            .collect();
        factsheet.api_endpoint_schema = Some(api_endpoints);

        PrintCommand::UnitTest.print_agent_message(
            self.attributes.position.as_str(),
            "Backend Code Unit Testing: Starting web server...",
        );
        let routes: Vec<String> = check_endpoints
            .iter()
            .map(|route| route.route.clone())
            .collect();
        let smoke_started = Instant::now();
        let probe = verification::probe(&project_dir, &routes).await?;
        trajectory::record_tool(
            &self.attributes.position,
            "runtime_smoke",
            &format!("port probe on {}", project_dir.display()),
            &probe.smoke.detail,
            probe.smoke.success,
            if probe.smoke.success {
                None
            } else {
                Some(probe.smoke.detail.clone())
            },
            smoke_started.elapsed().as_millis(),
        );
        for check in &probe.endpoints {
            let testing_msg = format!("Testing endpoint '{}'...", check.route);
            PrintCommand::UnitTest
                .print_agent_message(self.attributes.position.as_str(), testing_msg.as_str());
            if !check.success {
                let err_msg = format!(
                    "WARNING: Failed to call backend url endpoint {} ({})",
                    check.route,
                    check.error.clone().unwrap_or_else(|| {
                        check
                            .actual_status
                            .map(|status| status.to_string())
                            .unwrap_or_else(|| "no response".to_string())
                    })
                );
                PrintCommand::Issue
                    .print_agent_message(self.attributes.position.as_str(), err_msg.as_str());
            }
            trajectory::record_tool(
                &self.attributes.position,
                "endpoint_test",
                &check.url,
                &check
                    .actual_status
                    .map(|status| status.to_string())
                    .unwrap_or_else(|| "error".to_string()),
                check.success,
                check.error.clone(),
                0,
            );
        }

        save_api_endpoints(&api_endpoints_str)?;
        trajectory::set_verification(VerificationReport {
            build,
            smoke: Some(probe.smoke),
            endpoints: probe.endpoints,
        });
        PrintCommand::UnitTest.print_agent_message(
            self.attributes.position.as_str(),
            "Backend testing complete...",
        );
        self.attributes.state = AgentState::Finished;
        Ok(())
    }
}

#[async_trait]
impl SpecialFunctions for AgentBackendDeveloper {
    fn get_attributes_from_agent(&self) -> &BasicAgent {
        &self.attributes
    }

    async fn execute(&mut self, factsheet: &mut FactSheet) -> Result<(), AgentError> {
        while self.attributes.state != AgentState::Finished {
            match self.attributes.state {
                AgentState::Discovery => {
                    run_context::set_phase("Discovery");
                    let span = tracing::info_span!(
                        "agent.state",
                        agent = %self.attributes.position,
                        state = "Discovery"
                    );
                    async {
                        self.call_initial_backend_code(factsheet).await?;
                        self.attributes.state = AgentState::Working;
                        Ok::<(), AgentError>(())
                    }
                    .instrument(span)
                    .await?;
                }
                AgentState::Working => {
                    run_context::set_phase("Working");
                    let span = tracing::info_span!(
                        "agent.state",
                        agent = %self.attributes.position,
                        state = "Working"
                    );
                    async {
                        if self.bug_count == 0 {
                            self.call_improved_backend_code(factsheet).await?;
                        } else {
                            self.call_fix_code_bugs(factsheet).await?;
                        }
                        self.attributes.state = AgentState::UnitTesting;
                        Ok::<(), AgentError>(())
                    }
                    .instrument(span)
                    .await?;
                }
                AgentState::UnitTesting => {
                    run_context::set_phase("UnitTesting");
                    let span = tracing::info_span!(
                        "agent.state",
                        agent = %self.attributes.position,
                        state = "UnitTesting"
                    );
                    async { self.unit_test(factsheet).await }
                        .instrument(span)
                        .await?;
                }
                AgentState::Finished => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_backend_starts_in_discovery_with_no_bugs() {
        let agent = AgentBackendDeveloper::new();
        assert_eq!(agent.attributes.state, AgentState::Discovery);
        assert_eq!(agent.bug_count, 0);
        assert!(agent.bug_errors.is_none());
    }

    #[tokio::test]
    #[ignore = "calls OpenAI and builds the web template; set OPEN_AI_KEY and run with --ignored"]
    async fn tests_backend_developer() {
        let mut agent = AgentBackendDeveloper::new();
        let factsheet_str = r#"
      {
        "project_description": "build a website that fetches and tracks fitness progress with timezone information",
        "project_scope": {
          "is_crud_required": true,
          "is_user_login_and_logout": true,
          "is_external_urls_required": true
        },
        "external_urls": [
          "http://worldtimeapi.org/api/timezone"
        ],
        "backend_code": null,
        "api_endpoint_schema": null
      }"#;
        let mut factsheet: FactSheet = serde_json::from_str(factsheet_str).unwrap();
        agent.attributes.state = AgentState::Discovery;
        agent
            .execute(&mut factsheet)
            .await
            .expect("Failed to execute Backend Developer agent");
    }
}
