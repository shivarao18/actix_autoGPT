use crate::ai_functions::aifunc_architect::{print_project_scope, print_site_urls};
use crate::helpers::command_line::PrintCommand;
use crate::helpers::general::{ai_task_request_decoded, check_status_code, urls_to_keep};
use crate::models::agent_basic::basic_agent::{AgentState, BasicAgent};
use crate::models::agent_basic::basic_traits::BasicTraits;
use crate::models::agents::agent_traits::{AgentError, FactSheet, ProjectScope, SpecialFunctions};
use crate::run_context;
use crate::trajectory;

use async_trait::async_trait;
use reqwest::Client;
use std::time::{Duration, Instant};
use tracing::Instrument;

// Solutions Architect
#[derive(Debug)]
pub struct AgentSolutionArchitect {
    attributes: BasicAgent,
}

impl AgentSolutionArchitect {
    pub fn new() -> Self {
        let attributes = BasicAgent {
            objective: "Gathers information and design solutions for website development"
                .to_string(),
            position: "Solutions Architect".to_string(),
            state: AgentState::Discovery,
            memory: vec![],
        };
        Self { attributes }
    }

    async fn call_project_scope(
        &mut self,
        factsheet: &mut FactSheet,
    ) -> Result<ProjectScope, AgentError> {
        let msg_context = factsheet.project_description.clone();
        let ai_response: ProjectScope = ai_task_request_decoded::<ProjectScope>(
            msg_context,
            &self.attributes.position,
            get_function_string!(print_project_scope),
            print_project_scope,
        )
        .await?;

        factsheet.project_scope = Some(ai_response);
        // Original flow marks the scope step finished, then re-opens UnitTesting
        // when external URLs still have to be checked.
        self.attributes.update_state(AgentState::Finished);
        Ok(ai_response)
    }

    async fn call_determine_external_urls(
        &mut self,
        factsheet: &mut FactSheet,
        msg_context: String,
    ) -> Result<(), AgentError> {
        let ai_response: Vec<String> = ai_task_request_decoded::<Vec<String>>(
            msg_context,
            &self.attributes.position,
            get_function_string!(print_site_urls),
            print_site_urls,
        )
        .await?;

        factsheet.external_urls = Some(ai_response);
        self.attributes.state = AgentState::UnitTesting;
        Ok(())
    }

    async fn test_external_urls(&mut self, factsheet: &mut FactSheet) -> Result<(), AgentError> {
        let urls = factsheet
            .external_urls
            .clone()
            .ok_or("No URL object on factsheet")?;
        let mut exclude_urls: Vec<String> = Vec::new();
        let client = if run_context::skip_network_checks() {
            None
        } else {
            Some(Client::builder().timeout(Duration::from_secs(5)).build()?)
        };

        for url in &urls {
            let endpoint_str = format!("Testing URL Endpoint: {url}");
            PrintCommand::UnitTest
                .print_agent_message(self.attributes.position.as_str(), endpoint_str.as_str());
            let started = Instant::now();
            let span = tracing::info_span!(
                "tool.call",
                agent = %self.attributes.position,
                tool = "check_status_code",
                url = %url
            );
            let status_result = async {
                if run_context::skip_network_checks() {
                    Ok(200)
                } else {
                    check_status_code(client.as_ref().expect("client built"), url)
                        .await
                        .map_err(|err| err.to_string())
                }
            }
            .instrument(span)
            .await;

            match status_result {
                Ok(status_code) => {
                    trajectory::record_tool(
                        &self.attributes.position,
                        "check_status_code",
                        url,
                        &status_code.to_string(),
                        status_code == 200,
                        if status_code == 200 {
                            None
                        } else {
                            Some(format!("status {status_code}"))
                        },
                        started.elapsed().as_millis(),
                    );
                    if status_code != 200 {
                        exclude_urls.push(url.clone());
                    }
                }
                Err(err) => {
                    println!("Error checking {url}: {err}");
                    trajectory::record_tool(
                        &self.attributes.position,
                        "check_status_code",
                        url,
                        "",
                        false,
                        Some(err),
                        started.elapsed().as_millis(),
                    );
                }
            }
        }

        if !exclude_urls.is_empty() {
            factsheet.external_urls = Some(urls_to_keep(&urls, &exclude_urls));
        }
        self.attributes.state = AgentState::Finished;
        Ok(())
    }
}

#[async_trait]
impl SpecialFunctions for AgentSolutionArchitect {
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
                    let project_scope = async { self.call_project_scope(factsheet).await }
                        .instrument(span)
                        .await?;
                    if project_scope.is_external_urls_required {
                        self.call_determine_external_urls(
                            factsheet,
                            factsheet.project_description.clone(),
                        )
                        .await?;
                        self.attributes.state = AgentState::UnitTesting;
                    }
                }
                AgentState::UnitTesting => {
                    run_context::set_phase("UnitTesting");
                    let span = tracing::info_span!(
                        "agent.state",
                        agent = %self.attributes.position,
                        state = "UnitTesting"
                    );
                    async { self.test_external_urls(factsheet).await }
                        .instrument(span)
                        .await?;
                }
                _ => {
                    self.attributes.state = AgentState::Finished;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_architect_starts_in_discovery() {
        let agent = AgentSolutionArchitect::new();
        assert_eq!(agent.attributes.state, AgentState::Discovery);
        assert_eq!(
            agent.get_attributes_from_agent().position,
            "Solutions Architect"
        );
    }

    #[tokio::test]
    #[ignore = "calls OpenAI; set OPEN_AI_KEY and run with --ignored"]
    async fn tests_solution_architect() {
        let mut agent = AgentSolutionArchitect::new();
        let mut factsheet = FactSheet {
            project_description: "Build a full stack website with user login and logout that shows latest Forex prices".to_string(),
            project_scope: None,
            external_urls: None,
            backend_code: None,
            api_endpoint_schema: None,
        };
        agent
            .execute(&mut factsheet)
            .await
            .expect("Unable to execute Solutions Architect Agent");
        assert!(factsheet.project_scope.is_some());
        assert!(factsheet.external_urls.is_some());
    }
}
