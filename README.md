# stack-cli-util

This is a command-line utility meant for processing Moodle question-xml files and in particular [STACK](https://stack-assessment.org/)-questions inside them. This tool is intended to form a basis onto which one can build small "actions" that the tool can execute on materials, not many actions exist at this point.

## For who?

Meant for people working with Moodle question.xml files for example inside their [gitsync](https://github.com/maths/moodle-qbank_gitsync)-clones of question-banks.

## Current state of development

Changelog:
 - 0.1.0 the first release:
  - Can eliminate unused attachments.
  - Should be able to convert multilang and mlang2 to `[[lang]]`, not necessary perfectly for MCQ labels or content inside the logic.
  - Can extract some key text content from the XMLs for easier grepping.


# How to use?

**Before doing anything note that this tool updates the file it is given, it does not save a separate copy with the modifications. Make sure you have backups or use this on top of version control so that you can rollback if something goes wrong.**

Firstly, you need to get this tool, just clone this repository somewhere.

Then you need Rust and in particular the tool called [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html).

Once those both exist, you can go to the directory where you cloned the repository and start doing things, without installing the tool you can use commands like:
```
cargo run -- some.xml --files --write
```
Which would look for unused attachments to remove (that `--files` flag) and then write out the changes instead of just reporting its findings (that `--write` flag).

Running the command without any arguments should give some information, and calling with:
```
cargo run -- --help
```
Would naturally tell more.

## Actual installation

One can also compile this into a speedier binary and install it by running the following command in the cloned repository directory:
```
cargo install --path .
```
That should place the binary to `~/.cargo/bin/stack-cli-util` and after `rehash` or restarting the terminal you should then be able to use commands like:
```
stack-cli-util some.xml --stacklang
```
The benefits of installation are basically: a bit faster and no need to use that `--` in front of the arguments to use some arguments that happen to be shared with cargo itself.
