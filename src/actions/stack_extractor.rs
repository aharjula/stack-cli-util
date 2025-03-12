//! Extracts specific fields from STACK questions and prints them out
//! Intended for grepping with some more accuracy.

use position_preserving_moodle_question_xml_edit::{QParser, Question};
use position_preserving_moodle_question_xml_edit::stack::{STACKQuestion};
use crate::action::Action;


pub struct StackExtractor {
}

impl StackExtractor {
	/// Simple initialisation logic.
	pub fn new() -> StackExtractor {
		StackExtractor {}
	}
}


impl Action for StackExtractor {
	fn process(&mut self, question: &Question, parser: &mut QParser, flags: Vec<String>) -> (bool, Vec<String>) {
		let mut notes: Vec<String> = Vec::new();
		// First identify all the flags.
		let mut of: usize = 2;
		if flags.contains(&"of=0".to_string()) {
			of = 0;
		}
		if flags.contains(&"of=1".to_string()) {
			of = 1;
		}
		let mut parts:Vec<String> = Vec::new();
		let partflags: Vec<String> = flags.into_iter().filter(|a| a.starts_with("parts=")).map(|a| a[6..].to_string()).collect();
		for flag in partflags {
			for bit in flag.split(",") {
				parts.push(bit.trim().to_string());
			}
		}
		if parts.is_empty() {
			parts.push("qt".to_string());
		}

		let stack_question: STACKQuestion = parser.get_as_stack_question(question.index);

		let prefix: String = match of {
			1 => {format!("{}: ", question.index + 1)}
			2 => {format!("{}: ", question.name.unwrap_cdata())}
			_ => {String::new()}
		};

		// In some sort of an order.
		if parts.contains(&"qv".to_string()) || parts.contains(&"kv".to_string()) {
			let qv = stack_question.questionvariables.unwrap_cdata();
			if !qv.is_empty() {
				for line in qv.split("\n") {
					notes.push(format!("{prefix}{line}"));
				}
			}
		}
		if parts.contains(&"qt".to_string()) || parts.contains(&"ct".to_string()) {
			let qt = stack_question.questiontext.get_content().unwrap().unwrap_cdata();
			if !qt.is_empty() {
				for line in qt.split("\n") {
					notes.push(format!("{prefix}{line}"));
				}
			}
		}
		if parts.contains(&"gf".to_string()) || parts.contains(&"ct".to_string()) {
			let gf = stack_question.generalfeedback.get_content().unwrap().unwrap_cdata();
			if !gf.is_empty() {
				for line in gf.split("\n") {
					notes.push(format!("{prefix}{line}"));
				}
			}
		}

		for (_prtname, prt) in stack_question.prts.clone().into_iter() {
			if parts.contains(&"kv".to_string()) {
				let fv = prt.feedbackvariables.unwrap_cdata();
				if !fv.is_empty() {
					for line in fv.split("\n") {
						notes.push(format!("{prefix}{line}"));
					}	
				}
			}
			if parts.contains(&"ct".to_string()) {
				for i in 0..prt.nodes.len() {
					let tf = prt.nodes[i].truefeedback.clone().get_content().unwrap().unwrap_cdata();
					if !tf.is_empty() {
						for line in tf.split("\n") {
							notes.push(format!("{prefix}{line}"));
						}
					}
					let ff = prt.nodes[i].falsefeedback.clone().get_content().unwrap().unwrap_cdata();
					if !ff.is_empty() {
						for line in ff.split("\n") {
							notes.push(format!("{prefix}{line}"));
						}
					}
				}
			}
		}

		(false, notes)
	}


	fn name(&self) -> String {
		"STACK extractor".to_string()
	} 

	fn flag(&self) -> String {
		"stackextract".to_string()
	}

	fn description(&self) -> String {
		"Prints out parts of STACK questions, for grepping. To use, define the parts
and the output-format.

The output-format controls the prefix added to each line.
 --of=0 no prefix
 --of=1 question index
 --of=2 question name [default]

The outputted partnames can be given as a comma separated list, e.g --parts=qt,gf
 --parts=qt question text [default]
 --parts=gf general fedback
 --parts=qv question variables
 --parts=ct key castext, question text, general feedback and PRT feedbacks
 --parts=kv keyvals, question variables and PRT feedback variables
".to_string()
	}

	fn supports(&self, qtype: String) -> bool {
		// Only works for STACK.
		qtype == *"stack"
	}

	fn report(&self) -> Option<String> {
		None
	}
}