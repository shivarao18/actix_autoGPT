use crate::models::agent_basic::basic_traits::BasicTraits;
use crate::models::general::llm::Message;

#[derive(Debug, PartialEq)]
pub enum AgentState {
    Discovery,
    Working,
    UnitTesting,
    Finished,
}

#[derive(Debug)]
#[allow(dead_code)] // objective and memory are part of the agent interface; tests read them.
pub struct BasicAgent {
    pub objective: String,
    pub position: String,
    pub state: AgentState,
    pub memory: Vec<Message>,
}

impl BasicTraits for BasicAgent {
    fn new(objective: String, position: String) -> Self {
        Self {
            objective,
            position,
            state: AgentState::Discovery,
            memory: Vec::from([]),
        }
    }

    fn update_state(&mut self, new_state: AgentState) {
        self.state = new_state;
    }

    fn get_objective(&self) -> &String {
        &self.objective
    }

    fn get_position(&self) -> &String {
        &self.position
    }

    fn get_state(&self) -> &AgentState {
        &self.state
    }

    fn get_memory(&self) -> &Vec<Message> {
        &self.memory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_moves_from_discovery_to_finished() {
        let mut agent = BasicAgent::new("ship the api".to_string(), "Project Manager".to_string());
        assert_eq!(agent.get_state(), &AgentState::Discovery);
        assert_eq!(agent.get_position(), "Project Manager");
        assert_eq!(agent.get_objective(), "ship the api");
        assert!(agent.get_memory().is_empty());
        agent.update_state(AgentState::UnitTesting);
        assert_eq!(agent.get_state(), &AgentState::UnitTesting);
        agent.update_state(AgentState::Finished);
        assert_eq!(*agent.get_state(), AgentState::Finished);
    }
}
