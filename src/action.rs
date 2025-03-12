//! General definition for an action the CLI-tool can do.
use position_preserving_moodle_question_xml_edit::{QParser, Question};

pub trait Action {
	/// Looks into a given question, returns some notes as well as a boolean
	/// telling if something was done or would be done if the "write" flag was active.
	fn process(&mut self, question: &Question, parser: &mut QParser, flags: Vec<String>) -> (bool, Vec<String>);

	/// Gives a name for this action.
	/// e.g. "Attachement checker"
	fn name(&self) -> String;

	/// Gives a flag to be used when selecting this action to be in action.
	fn flag(&self) -> String;

	/// Longer description of the action.
	fn description(&self) -> String;

	/// Check if this action supports a given question type.
	fn supports(&self, qtype: String) -> bool;


	/// End report summarising what was or would have been done.
	fn report(&self) -> Option<String>;
}