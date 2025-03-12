mod action;
mod actions;

use position_preserving_moodle_question_xml_edit::{QParser, Question};
use crate::actions::attachments::FileAttachmentChecker;
use crate::actions::stack_lang::LangSyntaxConverter;
use crate::actions::stack_extractor::StackExtractor;
use crate::action::Action;


// All known action types.
enum Actions {
    A0(FileAttachmentChecker),
    A1(LangSyntaxConverter),
    A2(StackExtractor)
}

// I have not yet quite grokked the way to work with traits and vectors.
// So this is the mess to deal with them.
impl Action for Actions {
    fn process(&mut self, question: &Question, parser: &mut QParser, flags: Vec<String>) -> (bool, Vec<String>) {
        match self {
            Actions::A0(a) => {a.process(question, parser, flags)}
            Actions::A1(a) => {a.process(question, parser, flags)}
            Actions::A2(a) => {a.process(question, parser, flags)}
        }
    }

    fn name(&self) -> String {
        match self {
            Actions::A0(a) => {a.name()}
            Actions::A1(a) => {a.name()}
            Actions::A2(a) => {a.name()}
        }
    }

    fn flag(&self) -> String {
        match self {
            Actions::A0(a) => {a.flag()}
            Actions::A1(a) => {a.flag()}
            Actions::A2(a) => {a.flag()}
        }
    }

    fn description(&self) -> String {
        match self {
            Actions::A0(a) => {a.description()}
            Actions::A1(a) => {a.description()}
            Actions::A2(a) => {a.description()}
        }
    }

    fn supports(&self, qtype: String) -> bool {
        match self {
            Actions::A0(a) => {a.supports(qtype)}
            Actions::A1(a) => {a.supports(qtype)}
            Actions::A2(a) => {a.supports(qtype)}
        }
    }

    fn report(&self) -> Option<String> {
        match self {
            Actions::A0(a) => {a.report()}
            Actions::A1(a) => {a.report()}
            Actions::A2(a) => {a.report()}
        }
    }
}


fn main() {
    // Simple arguments.
    let args: Vec<String> = std::env::args().collect();
    let files: Vec<&String> = args[1..].iter().filter(|a| !a.starts_with("--")).collect();
    let flags: Vec<String> = args[1..].iter().filter(|a| a.starts_with("--")).map(|a| a[2..].to_string()).collect();
    
    // Init all known action types here.
    let mut actions: Vec<Actions> = vec![
        Actions::A0(FileAttachmentChecker::new()),
        Actions::A1(LangSyntaxConverter::new()),
        Actions::A2(StackExtractor::new())
    ];
    

    if flags.contains(&"help".to_string()) {
        for action in &mut actions {
            println!(" --{} {}", action.flag(), action.name());
            println!("{}", action.description());
            println!("\n");
        }
        return;
    }

    if args.len() == 1 || files.is_empty() || flags.is_empty() {
        println!("To use this tool you need to provide, both filename(s)
and some flags to define the actions to take.");
        println!("\nCurrently known actions:");
        println!(" --help Describes actions in some more detail");
        println!(" --write The general write flag to execute things not just report");
        
        for action in &actions {
            println!(" --{} {}", action.flag(), action.name());
        }
        
        return;
    }

    // Then process the files.
    for file_name in files {
        println!("Checking {}:", file_name.clone());
        let mut parser = QParser::load_xml_file(file_name.clone()).expect("Something bad with the file or file-name.");
        let mut any_changes: bool = false;
        let mut questions: Vec<Question> = parser.find_questions();
        for qi in 0..questions.len() {
            println!(" {:>3}/{} '{}':", qi + 1, questions.len(), questions[qi].name.unwrap_cdata());

            for action in &mut actions {
                // Because that lack of vector of these.
                if flags.contains(&action.flag()) && action.supports(questions[qi].qtype.clone()) {
                    let (changes, notes) = action.process(&questions[qi], &mut parser, flags.clone());
                    if changes {
                        any_changes = true;
                        // Something changed, the questions list is not going to be correct.
                        questions = parser.find_questions();
                    }
                    for note in notes {
                        println!("  {}", note);
                    }
                }
            }
        }
        if any_changes && flags.contains(&"write".to_string()) {
            match parser.save_to_file(file_name.clone()) {
                Ok(_) => {
                    // Nothing.
                },
                Err(e) => {
                    println!("Issues writing changes to '{}', stopping.", file_name.clone());
                    println!("{:?}", e);
                    return;
                }
            }
        }
    }

    // Provide end reports.
    for action in &actions {
        if flags.contains(&action.flag()) {
            let report: Option<String> = action.report();
            match report {
                Some(r) => {
                    println!("\nEnd report from '{}'", action.name());
                    println!("{}", r);
                },
                None => {
                    // Nothing to report.
                }
            }
        }
    }
}
