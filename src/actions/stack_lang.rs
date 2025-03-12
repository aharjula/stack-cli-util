//! Logic for converting old multilang and mlang2 language syntax to
//! STACKs own [[lang]]-syntax.
//!
//! For now does not target MCQ-options or inline CASText but tries 
//! to note usage in those.

use position_preserving_moodle_question_xml_edit::{QParser, Question, Change};
use position_preserving_moodle_question_xml_edit::stack::{STACKQuestion, STACKPath};
use stack_maxima_parser::parser::{StackMaximaParser, MPNode, MPNodeType, StackStringUsage};
use crate::action::Action;
use regex::Regex;

pub struct LangSyntaxConverter {
	// Random stats
	multilang_conversions: usize,
	mlang_conversions: usize,
	// Did we see MCQ inputs with difficult strings?
	saw_mcq: bool,
	// Did we see fragments in logic? i.e. start of lang-block but no end in the same "string".
	saw_logic_fragments: bool,
	// Somethign truly odd?
	saw_oddities: bool
}

impl LangSyntaxConverter {
	/// Simple initialisation logic.
	pub fn new() -> LangSyntaxConverter {
		LangSyntaxConverter {
			multilang_conversions: 0,
			mlang_conversions: 0,
			saw_mcq: false,
			saw_logic_fragments: false,
			saw_oddities: false
		}
	}
}


impl Action for LangSyntaxConverter {
	fn process(&mut self, question: &Question, parser: &mut QParser, flags: Vec<String>) -> (bool, Vec<String>) {
		let write = flags.contains(&"write".to_string());
		let mut notes: Vec<String> = Vec::new();
		let mut things_to_do: bool = false;

		// Get a better access to the contents.
		let mut stack_question: STACKQuestion = parser.get_as_stack_question(question.index);

		// Separate permutations to keep the pattern simple.
		let re_multilang_p1 = Regex::new("(?s)<span\\s+lang=\"([a-zA-Z0-9_\\-]+)\"\\s+class=\"multilang\"\\s*>(.*?)<\\/span>").unwrap();
		let re_multilang_p2 = Regex::new("(?s)<span\\s+class=\"multilang\"\\s+lang=\"([a-zA-Z0-9_\\-]+)\"\\s*>(.*?)<\\/span>").unwrap();

		let re_mlang = Regex::new("(?is)\\{\\s*mlang\\s+([a-z0-9_\\-\\s]*)\\}(.*?)\\{\\s*mlang\\s*\\}").unwrap();

		// Count actions also at question level.
		let mut qmod_count: usize = 0;

		// First iterate over all the big CASText blocks.
		for (_path, ct) in stack_question.get_castext_fields() {
			// Note by "currently", we mean that the current XML-serialisation
			// logic does not skip empty fields. Should that change the library
			// will probably simply not return those fields.
			let mut full_content = ct.clone().get_content().expect("These will currently always have content.").unwrap_cdata();
			let mut changes = false;
			for (whole, [lang, content]) in re_multilang_p1.captures_iter(&full_content.clone()).map(|caps| caps.extract()) {
				let start = full_content.find(whole).expect("Well it was found already.");
				let mut new_content = String::new();
				new_content.push_str(&full_content[0..start]);
				new_content.push_str(&format!("[[lang code='{lang}']]{content}[[/lang]]"));
				new_content.push_str(&full_content[whole.len()+start..]);
				full_content = new_content;
				changes = true;
				self.multilang_conversions += 1;
				qmod_count += 1;
			}
			for (whole, [lang, content]) in re_multilang_p2.captures_iter(&full_content.clone()).map(|caps| caps.extract()) {
				let start = full_content.find(whole).expect("Well it was found already.");
				let mut new_content = String::new();
				new_content.push_str(&full_content[0..start]);
				new_content.push_str(&format!("[[lang code='{lang}']]{content}[[/lang]]"));
				new_content.push_str(&full_content[whole.len()+start..]);
				full_content = new_content;
				changes = true;
				self.multilang_conversions += 1;
				qmod_count += 1;
			}

			for (whole, [lang, content]) in re_mlang.captures_iter(&full_content.clone()).map(|caps| caps.extract()) {
				let start = full_content.find(whole).expect("Well it was found already.");
				let mut new_content = String::new();
				let trimmedlang = lang.trim();
				new_content.push_str(&full_content[0..start]);
				new_content.push_str(&format!("[[lang code='{trimmedlang}']]{content}[[/lang]]"));
				new_content.push_str(&full_content[whole.len()+start..]);
				full_content = new_content;
				changes = true;
				self.mlang_conversions += 1;
				qmod_count += 1;
			}

			if changes {
				things_to_do = true;
				if write {
					let change: Change = Change::cdata_wrapped_version(ct.get_content().unwrap().clone(), full_content);
					parser.register_change(change);
				}
			}
		}

		// The inputs may need to touch question-variables. So we will extract them here.
		let mut question_variables = stack_question.questionvariables.unwrap_cdata();

		// Then check for MCQ-inputs.
		for (inputname, input) in stack_question.inputs.clone().into_iter() {
			match input.r#type.unwrap_cdata().as_str() {
				"checkbox" | "dropdown" | "radio" => {
					notes.push(format!("  - MCQ input '{}'.", inputname));
					// Check for locally, in TANS defined options.
					let mut mparser = StackMaximaParser::new_no_insertions();
					let mut rawtans = input.tans.unwrap_cdata();
					let tans: Option<MPNode> = mparser.parse(rawtans.clone());

					if tans.is_none() {
						notes.push("   + Issues parsing the `tans`-value.".to_string());
					} else if let MPNodeType::Root(statements,_,_) = tans.unwrap().value {
						if let MPNodeType::Statement(expr, _) = &statements[0].value {
							// The question is whether the "expr" is an identifier
							// or a list.
							match &expr.value {
								MPNodeType::Identifier(id) => {
									notes.push(format!("   + Options defined in question-variables as '{id}'."));
								},
								MPNodeType::List(items) => {
									// Now there is a possibility that the list has labels.
									// And those might have some content.
									// We will iterate over them in reverse order, 
									// to allow modifications to be made in sensible order.
									for optionlist in items.iter().rev() {
										if let MPNodeType::List(inner_items) = &optionlist.value {
											if inner_items.len() > 2 {
												// Extract possible strings.
												let strings = inner_items[2].extract_stack_string_usage(StackStringUsage::Unknown);
												if strings.len() > 1 {
													notes.push("   + Complicated label, could not inpect fragments.".to_string());
												} else if strings.len() == 1 {
													// So a single string, this we can work with.
													let stringvalue = if let MPNodeType::String(v) = &strings[0].1.value {v.clone()} else {String::new()};
													let mut modified = stringvalue.clone();

													for (whole, [lang, content]) in re_multilang_p1.captures_iter(&modified.clone()).map(|caps| caps.extract()) {
														let start = modified.find(whole).expect("Well it was found already.");
														let mut new_content = String::new();
														new_content.push_str(&modified[0..start]);
														new_content.push_str(&format!("[[lang code='{lang}']]{content}[[/lang]]"));
														new_content.push_str(&modified[whole.len()+start..]);
														modified = new_content;
														self.multilang_conversions += 1;
														qmod_count += 1;
													}
													for (whole, [lang, content]) in re_multilang_p2.captures_iter(&modified.clone()).map(|caps| caps.extract()) {
														let start = modified.find(whole).expect("Well it was found already.");
														let mut new_content = String::new();
														new_content.push_str(&modified[0..start]);
														new_content.push_str(&format!("[[lang code='{lang}']]{content}[[/lang]]"));
														new_content.push_str(&modified[whole.len()+start..]);
														modified = new_content;
														self.multilang_conversions += 1;
														qmod_count += 1;
													}

													for (whole, [lang, content]) in re_mlang.captures_iter(&modified.clone()).map(|caps| caps.extract()) {
														let start = modified.find(whole).expect("Well it was found already.");
														let mut new_content = String::new();
														let trimmedlang = lang.trim();
														new_content.push_str(&modified[0..start]);
														new_content.push_str(&format!("[[lang code='{trimmedlang}']]{content}[[/lang]]"));
														new_content.push_str(&modified[whole.len()+start..]);
														modified = new_content;
														self.mlang_conversions += 1;
														qmod_count += 1;
													}

													if modified != stringvalue {
														// So mod this options label.
														match strings[0].0 {
															StackStringUsage::ListElement(ind) => {
																if ind == 2 {
																	// Third element of a list, don't touch first elements in particular.
																	// Not inline CASText. Will need to be.
																	let mut newstring: String = String::new();
																	newstring.push_str(&rawtans[0..strings[0].1.position.startbyte]);
																	newstring.push_str("castext(\"");
																	newstring.push_str(&modified.replace("\\","\\\\").replace("\"","\\\""));
																	newstring.push_str("\")");
																	newstring.push_str(&rawtans[strings[0].1.position.endbyte..rawtans.len()]);
																	rawtans = newstring;
																} else {
																	self.saw_mcq = true;
																	notes.push("   + Localisation spotted in value not in label? Not touching this.".to_string());
																}
															},
															StackStringUsage::CASText => {
																// Already inline CASText. This branch should not happen as `tans` does not support direct inline CASText.
																let mut newstring: String = String::new();
																newstring.push_str(&rawtans[0..strings[0].1.position.startbyte]);
																newstring.push('"');
																newstring.push_str(&modified.replace("\\","\\\\").replace("\"","\\\""));
																newstring.push('"');
																newstring.push_str(&rawtans[strings[0].1.position.endbyte..rawtans.len()]);
																rawtans = newstring;
															},
															_ => {
																panic!("Unexpected string use declared.");
															}
														}
													}
												} else {
													self.saw_mcq = true;
													notes.push("   + Stringless custom-label, probably defined elsewhere.".to_string());
												}
											} else {
												// No custom label
											}
										} else {
											self.saw_mcq = true;
											notes.push("   + Odd definition of options, please provide sample to tool devs.".to_string());
										}
									}
								},
								_ => {
									self.saw_mcq = true;
									notes.push("   + Found unexpected expression-type in the `tans`-field.".to_string());
								}
							}
						}
					} else {
						panic!("Parser returned something odd");
					}

					if input.tans.unwrap_cdata() != rawtans {
						notes.push("   + Modified locally defined `tans`-value.".to_string());
						notes.push("   + Transferred definition to the end of question-variables. As inline CASText requires that.".to_string());
						things_to_do = true;
						if write {
							let mut label: String = String::from("auto_relocated_");
							label.push_str(&input.name.unwrap_cdata());
							label.push_str("_options");
							question_variables.push_str(&format!("\n\n{label}: {rawtans};"));
							let change: Change = Change::cdata_wrapped_version(input.tans.clone(), label);
							parser.register_change(change);
						}
					}

					
				},
				_ => {
					// Not MCQ just ignore
				}
			}
		}

		// If question-variables changed during MCQ-processing commit that.
		if stack_question.questionvariables.unwrap_cdata() != question_variables {
			let change: Change = Change::cdata_wrapped_version(stack_question.questionvariables.clone(), question_variables);
			if write {
				parser.register_change(change);
				// If we modified the question variables the following steps will need new
				// version of them.
				stack_question = parser.get_as_stack_question(question.index);
			}
		}

		// Check rest of the logic.
		for (path, keyval) in stack_question.get_keyval_fields() {
			let mut unwrapped: String = keyval.unwrap_cdata();
			if unwrapped.contains("mlang") || unwrapped.contains("multilang") {
				// Depending on where we are we might have different level of trust
				// on what we target.
				// e.g. do we feel confortable to target third elements of lists?
				let mut target_third_elements = false;
				// Or inline castext, although who would use other localisation in it.
				let target_inline_castext = true;
				match path {
					STACKPath::Root(_) => {
						// Only question variables in root.
						notes.push("  - Has specific sequences in question variables.".to_string());
						target_third_elements = true;
					},
					STACKPath::PRT(name,_) => {
						// Only feedback-variables in PRTs.
						notes.push(format!("  - Has specific sequences in {} feedback variables.", name));
					}
					_ => {
						panic!("Unexpected new type of keyval block! Maybe this logic needs to be reworked.");
					}					
				}

				let mut mparser = StackMaximaParser::new_with_insert_semicolons();
				let parsedkeyval: Option<MPNode> = mparser.parse(unwrapped.clone());
				let stringuses = parsedkeyval.expect("Something was syntactically broken.").extract_stack_string_usage(StackStringUsage::Unknown);

				for (typeofuse, stringnode) in stringuses.iter().rev() {
					if let MPNodeType::String(value) = &stringnode.value {
						if !(value.contains("mlang") || value.contains("multilang")) {
							continue;
						}
						let mut modified = value.clone();
						
						for (whole, [lang, content]) in re_multilang_p1.captures_iter(&modified.clone()).map(|caps| caps.extract()) {
							let start = modified.find(whole).expect("Well it was found already.");
							let mut new_content = String::new();
							new_content.push_str(&modified[0..start]);
							new_content.push_str(&format!("[[lang code='{lang}']]{content}[[/lang]]"));
							new_content.push_str(&modified[whole.len()+start..]);
							modified = new_content;
							self.multilang_conversions += 1;
							qmod_count += 1;
						}
						for (whole, [lang, content]) in re_multilang_p2.captures_iter(&modified.clone()).map(|caps| caps.extract()) {
							let start = modified.find(whole).expect("Well it was found already.");
							let mut new_content = String::new();
							new_content.push_str(&modified[0..start]);
							new_content.push_str(&format!("[[lang code='{lang}']]{content}[[/lang]]"));
							new_content.push_str(&modified[whole.len()+start..]);
							modified = new_content;
							self.multilang_conversions += 1;
							qmod_count += 1;
						}

						for (whole, [lang, content]) in re_mlang.captures_iter(&modified.clone()).map(|caps| caps.extract()) {
							let start = modified.find(whole).expect("Well it was found already.");
							let mut new_content = String::new();
							let trimmedlang = lang.trim();
							new_content.push_str(&modified[0..start]);
							new_content.push_str(&format!("[[lang code='{trimmedlang}']]{content}[[/lang]]"));
							new_content.push_str(&modified[whole.len()+start..]);
							modified = new_content;
							self.mlang_conversions += 1;
							qmod_count += 1;
						}


						match typeofuse {
							StackStringUsage::CASText => {
								if modified == *value {
									self.saw_logic_fragments = true;
								} else if target_inline_castext {
									// Simply update the string, surely these are not escaped things?
									// Balancing between edge cases is difficult.
									let mut newstring: String = String::new();
									newstring.push_str(&unwrapped[0..stringnode.position.startbyte]);
									newstring.push('"');
									newstring.push_str(&modified.replace("\\","\\\\").replace("\"","\\\""));
									newstring.push('"');
									newstring.push_str(&unwrapped[stringnode.position.endbyte..unwrapped.len()]);
									unwrapped = newstring;
								}
							}
							StackStringUsage::CASTextConcat => {
								if modified == *value {
									self.saw_logic_fragments = true;
								} else if target_inline_castext {
									// In a string argument of CASTextConcat!? 
									// Odd but we can turn that to inline CASText.
									let mut newstring: String = String::new();
									newstring.push_str(&unwrapped[0..stringnode.position.startbyte]);
									newstring.push_str("castext(\"");
									newstring.push_str(&modified.replace("\\","\\\\").replace("\"","\\\""));
									newstring.push_str("\")");
									newstring.push_str(&unwrapped[stringnode.position.endbyte..unwrapped.len()]);
									unwrapped = newstring;
								}
							}
							StackStringUsage::CompiledCASText(_) => {
								if modified == *value {
									self.saw_logic_fragments = true;
								} else {
									self.saw_oddities = true;
									notes.push("   + Spotted localisation in something looking like externally compiled CASText. Not touching.".to_string());
								}
							}
							StackStringUsage::Include | StackStringUsage::IncludeContrib => {
								// These are ignored, maybe the specific substrings just happen to be in the url...
							}
							StackStringUsage::ListElement(ind) => {
								if modified == *value {
									self.saw_logic_fragments = true;
								} else if target_third_elements && *ind == 2 {
									notes.push("   + Spotted a likely MCQ-option label, turning it to inline CASText, this might break things.".to_string());
									let mut newstring: String = String::new();
									newstring.push_str(&unwrapped[0..stringnode.position.startbyte]);
									newstring.push_str("castext(\"");
									newstring.push_str(&modified.replace("\\","\\\\").replace("\"","\\\""));
									newstring.push_str("\")");
									newstring.push_str(&unwrapped[stringnode.position.endbyte..unwrapped.len()]);
									unwrapped = newstring;
								} else {
									notes.push("   + Spotted localisation in string not directly identified as a safe target.".to_string());
								}
							}
							StackStringUsage::Unknown => {
								if modified == *value {
									self.saw_logic_fragments = true;
								} else {
									notes.push("   + Spotted localisation in string not directly identified as a safe target.".to_string());
								}
							}
						}
					}
				}

				// Are there still bits with those.
				if unwrapped.contains("mlang") || unwrapped.contains("multilang") {
					notes.push("   + Localisation possibly used in areas not felt safe to modify.".to_string());
				}
			}
			// Did we change something?
			if keyval.unwrap_cdata() != unwrapped {
				let change: Change = Change::cdata_wrapped_version(keyval.clone(), unwrapped);
				if write {
					parser.register_change(change);
				}	
			}
		}

		if qmod_count > 0 {
			if write {
				notes.push(format!(" Converted {} other lang syntax uses to `[[lang]]`.", qmod_count));
			} else {
				notes.push(format!(" Could convert {} other lang syntax uses to `[[lang]]`.", qmod_count));
			}
		}

		(things_to_do, notes)
	}

	fn name(&self) -> String {
		"STACK [[lang]]-converter".to_string()
	} 

	fn flag(&self) -> String {
		"stacklang".to_string()
	}

	fn description(&self) -> String {
		"Converts old multilang and mlang2 localisation syntax to the STACK CASText
[[lang]]-block syntax.

Tries to also fix MCQ-labels, but won't be too aggressive trying to convert
\"strings\" in keyvals to castext.".to_string()
	}

	fn supports(&self, qtype: String) -> bool {
		// Only works for STACK.
		qtype == *"stack"
	}

	fn report(&self) -> Option<String> {
		if self.multilang_conversions == 0 &&
			self.mlang_conversions == 0 &&
			!self.saw_mcq &&
			!self.saw_logic_fragments &&
			!self.saw_oddities {
			// Did nothing.
			return None;
		}
		let mut result: String = String::new();
		if self.multilang_conversions > 0 {
			result.push_str(&format!("Could replace {} '<span class=\"multilang\">' uses.\n", self.multilang_conversions));
		}
		if self.mlang_conversions > 0 {
			result.push_str(&format!("Could replace {} '{{mlang}}' uses.\n", self.mlang_conversions));
		}
		if self.saw_mcq {
			result.push_str("Saw something odd in MCQ-inputs, check those.\n");
		}
		if self.saw_logic_fragments {
			result.push_str("Saw fragmented localisation syntax in logic, cannot fix that.\n");
		}
		if self.saw_oddities {
			result.push_str("Saw truly odd, did not know what to do.\n");
		}
		Some(result)
	}
}