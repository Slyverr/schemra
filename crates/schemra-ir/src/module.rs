use crate::{ScenarioIr, VisualIr};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModuleIr {
    pub visuals: Vec<VisualIr>,
    pub scenarios: Vec<ScenarioIr>,
}
