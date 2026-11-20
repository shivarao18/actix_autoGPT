use crate::ai_functions::aifunc_managing::convert_user_input_to_goal;
use crate::helpers::general::ai_task_request;
use crate::models::agent_basic::basic_agent::{AgentState, BasicAgent};
use crate::models::agents::agent_architect::AgentSolutionArchitect;
use crate::models::agents::agent_backend::AgentBackendDeveloper;
use crate::models::agents::agent_traits::{AgentError, FactSheet, SpecialFunctions};
use tracing::Instrument;

#[derive(Debug)]
pub struct ManagingAgent {
    _attributes: BasicAgent,
    factsheet: FactSheet,
    agents: Vec<Box<dyn SpecialFunctions>>,
}

impl ManagingAgent {
    pub async fn new(usr_req: String) -> Result<Self, AgentError> {
        let position = "Project Manager".to_string();
        let attributes = BasicAgent {
            objective: "Manage agents who are building an excellent website for the user"
                .to_string(),
            position: position.clone(),
            state: AgentState::Discovery,
            memory: vec![],
        };

        let project_description = ai_task_request(
            usr_req,
            &position,
            get_function_string!(convert_user_input_to_goal),
            convert_user_input_to_goal,
        )
        .await?;

        let factsheet = FactSheet {
            project_description,
            project_scope: None,
            external_urls: None,
            backend_code: None,
            api_endpoint_schema: None,
        };

        Ok(Self {
            _attributes: attributes,
            factsheet,
            agents: Vec::new(),
        })
    }

    pub fn factsheet(&self) -> &FactSheet {
        &self.factsheet
    }

    fn add_agent(&mut self, agent: Box<dyn SpecialFunctions>) {
        self.agents.push(agent);
    }

    fn create_agents(&mut self) {
        self.add_agent(Box::new(AgentSolutionArchitect::new()));
        self.add_agent(Box::new(AgentBackendDeveloper::new()));
    }

    pub async fn execute_project(&mut self) -> Result<(), AgentError> {
        self.create_agents();
        for agent in &mut self.agents {
            let name = agent.get_attributes_from_agent().position.clone();
            let span = tracing::info_span!("agent.step", agent = %name);
            agent.execute(&mut self.factsheet).instrument(span).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "calls OpenAI and builds generated code; set OPEN_AI_KEY and run with --ignored"]
    async fn tests_managing_agent() {
        let usr_request = "need a full stack app that fetches and tracks my fitness progress. Needs to include timezone info from the web.";
        let mut managing_agent = ManagingAgent::new(usr_request.to_string())
            .await
            .expect("Error creating Managing Agent");
        managing_agent.execute_project().await.expect("project");
        assert!(!managing_agent.factsheet().project_description.is_empty());
    }
}
