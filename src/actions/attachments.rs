//! Logic for detecting unused attachment files present in questions.
//! If executed with "write"-flag will remove unused ones.

use position_preserving_moodle_question_xml_edit::{QParser, Question, ContentType, Change, ContentRef};
use crate::action::Action;
use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;
use urlencoding::decode as url_decode;

pub struct FileAttachmentChecker {
	total_files: usize,
	total_bytes: usize,
	removed_files: usize,
	removed_bytes: usize,
	all_ok: bool
}

impl FileAttachmentChecker {
	/// Simple initialisation logic.
	pub fn new() -> FileAttachmentChecker {
		FileAttachmentChecker {
			total_files: 0,
			total_bytes: 0,
			removed_files: 0,
			removed_bytes: 0,
			all_ok: true
		}
	}
}

impl Action for FileAttachmentChecker {
	fn process(&mut self, question: &Question, parser: &mut QParser, flags: Vec<String>) -> (bool, Vec<String>) {
		let write = flags.contains(&"write".to_string());
		let mut notes: Vec<String> = Vec::new();
		let mut things_to_do: bool = false;

		// iff A\B == ø -> delete(B\A) otherwise something is wrong.
		//
		// Basically, identify @@PLUGINFILES@@ referenced by the material and
		// the files present. If all of the former are present in the latter
		// remove from the latter those that are not in the former. Otherwise,
		// assume that the identification of the former is broken.
		
		let re_dq = Regex::new("\"@@PLUGINFILE@@([^\"]*)\"").unwrap();
		let re_q = Regex::new("'@@PLUGINFILE@@([^\"]*)'").unwrap();

		// From raw to url-decoded.
		let mut a: HashMap<String, String> = HashMap::new();
		
		for (_, [f]) in re_dq.captures_iter(question.whole_element.content.as_str()).map(|caps| caps.extract()) {
			// Note that some people have been adding get parameters to attachement file URLs.
			// We assume that these have been used to deal with caches and ignore them.
			let getless = f.split('?').next().unwrap();
    		a.insert(f.to_string().clone(), url_decode(getless).unwrap().to_string());
		}
		for (_, [f]) in re_q.captures_iter(question.whole_element.content.as_str()).map(|caps| caps.extract()) {
			let getless = f.split('?').next().unwrap();
    		a.insert(f.to_string().clone(), url_decode(getless).unwrap().to_string());
		}
		let mut matched: HashSet<String> = HashSet::new();

		// Find the files.
		let file_elements = parser.get_elements(question.index, vec!["file".to_string()]);
		let mut to_delete: Vec<(String, ContentRef)> = Vec::new();
		if file_elements.is_empty() && !a.is_empty() {
			notes.push(" WARNING! Question has references to files but not files present.".to_string());
			self.all_ok = false;
		} else {
			for file_element in &file_elements {
                if let ContentType::Element(_name, whole_element_ref, _attributes_and_content) = file_element {
                	let attachment_path: String = file_element.clone().get_attr("path".to_string()).expect("File element must have a 'path' attribute.").basic_entity_decode();
                    let attachment_name: String = file_element.clone().get_attr("name".to_string()).expect("File element must have a 'name' attribute.").basic_entity_decode();
                    let name = format!("{attachment_path}{attachment_name}");
                    if a.contains_key(&name) {
                    	// Direct match.
                    	matched.insert(name.clone());
                    } else {
                    	let mut found = false;
                    	// Maybe one of the urldecoded ones.
                    	for (raw, decoded) in a.clone().into_iter() {
                    		if decoded == name {
                    			found = true;
                    			matched.insert(raw.clone());
                    		}
                    	}
                    	if !found {
                    		// No reference seen. So push to be deleted.
                    		to_delete.push((name,whole_element_ref.clone()));
                    		// Do some bookkeepping.
                    		self.removed_files += 1;
                    		self.removed_bytes += whole_element_ref.content.len();
                    	}
                    }
					// Do some bookkeepping.
            		self.total_files += 1;
            		self.total_bytes += whole_element_ref.content.len();
                } else {
                	panic!("Unexpected ContentType received as a search result.");
                }
            }
		}

		// Now did we match all?
		if matched.len() == a.len() {
			// So all matched can delete the ones.
			if !to_delete.is_empty() {
				things_to_do = true;
				if write {
					for (name, cref) in &to_delete {
						notes.push(format!(" Deleting unused file '{}', saving {} bytes.", name, cref.content.len()));
						let change: Change = Change::new(cref.clone(), "".to_string());
						parser.register_change(change);
					}
				} else {
					for (name, cref) in &to_delete {
						notes.push(format!(" Could delete unused file '{}', and save {} bytes.", name, cref.content.len()));
					}
				}
			}
		} else {
			for (raw, _decoded) in a.clone().into_iter() {
				if !matched.contains(&raw) {
					notes.push(format!(" WARNING! References '{raw}', which was not matched."));
				}
			}

			// Did not find matches for all.
			self.all_ok = false;
		}

		(things_to_do, notes)
	}

	fn name(&self) -> String {
		"Excess attachment remover".to_string()
	} 

	fn flag(&self) -> String {
		"files".to_string()
	}

	fn description(&self) -> String {
		"Unused attachement files take room and removing them is difficult, this tool
detects and can remove such files present in the question.xml.

The primary source for such files is duplication of questions, expect the tool
to often list the very same files for many questions in a row.

Note that this tool will not do de-duplication or access right tuning so the
end result might still not be the smallest possible.".to_string()
	}

	fn supports(&self, _qtype: String) -> bool {
		// All Moodle question-types should use Moodle pluginfiles...
		true
	}

	fn report(&self) -> Option<String> {
		// Maybe tell how many files and how much space.
		if self.total_files > 0 {
			let mut result: String = format!("Saw {} files of which {} could be removed.
In total those files take {} bytes of room and the removable ones {}.",
				 self.total_files, self.removed_files, self.total_bytes, self.removed_bytes);
			if !self.all_ok {
				result.push_str("

NOTE! That some questions had references to files that could not be matched by current logic.
Any extra files those questions might have had were not removed.");
			}
			Some(result)
		} else {
			None	
		}
	}
}