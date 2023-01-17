use std::collections::HashSet;

use regex::Regex;

#[derive(Debug)]
pub enum PipelineStep {
    Filter(Regex),
    Lower,
    Upper,
    Trim,
    Dedupe(HashSet<String>, usize),
    Append(String),
    Prepend(String),
}
pub struct Pipeline {
    steps: Vec<PipelineStep>,
}

impl Pipeline {
    pub fn build_pipeline<T: AsRef<str>>(mut tokens: &[T]) -> Result<Pipeline, &'static str> {
        let mut steps: Vec<PipelineStep> = vec![];

        loop {
            if tokens.len() == 0 {
                break;
            }

            let command = tokens[0].as_ref();
            let argument = tokens.get(1).map(|r| r.as_ref());

            let step = match command.to_lowercase().as_str() {
                "filter" => {
                    tokens = &tokens[1..];
                    let regex = Regex::new(argument.ok_or("Missing regular expression")?)
                        .map_err(|_| "Invalid regular expression")?;

                    PipelineStep::Filter(regex)
                }
                "lower" => PipelineStep::Lower,
                "upper" => PipelineStep::Upper,
                "trim" => PipelineStep::Trim,
                "dedupe" => PipelineStep::Dedupe(HashSet::new(), 0),
                "append" => {
                    tokens = &tokens[1..];
                    PipelineStep::Append(argument.ok_or("Missing suffix")?.to_string())
                }
                "prepend" => {
                    tokens = &tokens[1..];
                    PipelineStep::Prepend(argument.ok_or("Missing prefix")?.to_string())
                }
                _ => Err("Invalid command specified")?,
            };

            tokens = &tokens[1..];
            steps.push(step);
        }

        if steps.len() < 0 {
            Err("No commands specified")
        } else {
            Ok(Pipeline { steps })
        }
    }

    pub fn apply(&mut self, line: &str) -> Option<String> {
        let mut output = line.to_string();

        for step in self.steps.iter_mut() {
            output = match step {
                PipelineStep::Filter(regex) => {
                    if !regex.is_match(&output) {
                        return None;
                    }

                    output
                }
                PipelineStep::Append(suffix) => output + suffix,
                PipelineStep::Prepend(prefix) => prefix.to_owned() + &output,
                PipelineStep::Dedupe(ref mut dupes, stored) => {
                    if dupes.contains(&output) {
                        return None;
                    } else {
                        dupes.insert(output.to_string());
                        *stored = *stored + output.len();

                        output.to_string()
                    }
                }
                PipelineStep::Lower => output.to_lowercase(),
                PipelineStep::Upper => output.to_uppercase(),
                PipelineStep::Trim => output.trim().to_string(),
            }
        }

        Some(output)
    }

    pub fn get_memory(&self) -> usize {
        let mut memory = 0;
        for step in self.steps.iter() {
            memory += match step {
                PipelineStep::Dedupe(_, bytes) => *bytes,
                _ => 0,
            }
        }

        memory
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use regex::Regex;

    use super::{Pipeline, PipelineStep};

    #[test]
    fn build_pipeline_rejects_zero_commands() {
        //+ Arrange
        let tokens: Vec<&str> = vec![];

        //+ Act
        let pipeline = Pipeline::build_pipeline(&tokens);

        //+ Assert
        assert!(pipeline.is_err());
        assert_eq!(pipeline.err().unwrap(), "No commands specified");
    }

    #[test]
    fn build_pipeline_parses_filter_command() -> Result<(), String> {
        //+ Arrange
        let tokens: Vec<&str> = vec!["filter", ".+"];

        //+ Act
        let pipeline = Pipeline::build_pipeline(&tokens)?;

        //+ Assert
        assert_steps(
            &pipeline,
            &[PipelineStep::Filter(Regex::new(".+").unwrap())],
        )
    }

    #[test]
    fn build_pipeline_rejects_invalid_regex() {
        //+ Arrange
        let tokens: Vec<&str> = vec!["filter", r"\"];

        //+ Act
        let pipeline = Pipeline::build_pipeline(&tokens);

        //+ Assert
        assert!(pipeline.is_err());
        assert_eq!(pipeline.err().unwrap(), "Invalid regular expression");
    }

    #[test]
    fn build_pipeline_parses_append_command() -> Result<(), String> {
        //+ Arrange
        let tokens: Vec<&str> = vec!["append", "foo"];

        //+ Act
        let pipeline = Pipeline::build_pipeline(&tokens)?;

        //+ Assert
        assert_steps(&pipeline, &[PipelineStep::Append("foo".to_string())])
    }

    #[test]
    fn build_pipeline_rejects_missing_suffix() {
        //+ Arrange
        let tokens: Vec<&str> = vec!["append"];

        //+ Act
        let pipeline = Pipeline::build_pipeline(&tokens);

        //+ Assert
        assert!(pipeline.is_err());
        assert_eq!(pipeline.err().unwrap(), "Missing suffix");
    }

    #[test]
    fn build_pipeline_parses_prepend_command() -> Result<(), String> {
        //+ Arrange
        let tokens: Vec<&str> = vec!["prepend", "foo"];

        //+ Act
        let pipeline = Pipeline::build_pipeline(&tokens)?;

        //+ Assert
        assert_steps(&pipeline, &[PipelineStep::Prepend("foo".to_string())])
    }

    #[test]
    fn build_pipeline_rejects_missing_prefix() {
        //+ Arrange
        let tokens: Vec<&str> = vec!["prepend"];

        //+ Act
        let pipeline = Pipeline::build_pipeline(&tokens);

        //+ Assert
        assert!(pipeline.is_err());
        assert_eq!(pipeline.err().unwrap(), "Missing prefix");
    }

    #[test]
    fn build_pipeline_parses_dedupe_command() -> Result<(), String> {
        //+ Arrange
        let tokens: Vec<&str> = vec!["dedupe"];

        //+ Act
        let pipeline = Pipeline::build_pipeline(&tokens)?;

        //+ Assert
        assert_steps(&pipeline, &[PipelineStep::Dedupe(HashSet::new())])
    }

    #[test]
    fn build_pipeline_parses_lower_command() -> Result<(), String> {
        //+ Arrange
        let tokens: Vec<&str> = vec!["lower"];

        //+ Act
        let pipeline = Pipeline::build_pipeline(&tokens)?;

        //+ Assert
        assert_steps(&pipeline, &[PipelineStep::Lower])
    }

    #[test]
    fn build_pipeline_parses_upper_command() -> Result<(), String> {
        //+ Arrange
        let tokens: Vec<&str> = vec!["upper"];

        //+ Act
        let pipeline = Pipeline::build_pipeline(&tokens)?;

        //+ Assert
        assert_steps(&pipeline, &[PipelineStep::Upper])
    }

    #[test]
    fn build_pipeline_parses_trim_command() -> Result<(), String> {
        //+ Arrange
        let tokens: Vec<&str> = vec!["trim"];

        //+ Act
        let pipeline = Pipeline::build_pipeline(&tokens)?;

        //+ Assert
        assert_steps(&pipeline, &[PipelineStep::Trim])
    }

    #[test]
    fn build_pipeline_parses_multiple_commands() -> Result<(), String> {
        //+ Arrange
        let tokens: Vec<&str> = vec!["lower", "upper", "filter", ".+", "prepend", "hello"];

        //+ Act
        let pipeline = Pipeline::build_pipeline(&tokens)?;

        //+ Assert
        assert_steps(
            &pipeline,
            &[
                PipelineStep::Lower,
                PipelineStep::Upper,
                PipelineStep::Filter(Regex::new(".+").unwrap()),
                PipelineStep::Prepend("hello".to_string()),
            ],
        )
    }

    #[test]
    fn apply_dedupe_hides_duplicates() {
        //+ Arrange
        let mut pipeline = Pipeline::build_pipeline(&["lower", "dedupe"]).unwrap();

        //+ Act + Assert
        assert_eq!(pipeline.apply("fOo"), Some("foo".to_string()));
        assert_eq!(pipeline.apply("fOo"), None);
    }

    fn assert_steps(pipeline: &Pipeline, expected_steps: &[PipelineStep]) -> Result<(), String> {
        assert_eq!(pipeline.steps.len(), expected_steps.len());

        for (index, actual_step) in expected_steps.iter().enumerate() {
            assert_eq!(actual_step, &expected_steps[index]);
        }

        Ok(())
    }
}

impl PartialEq for PipelineStep {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Filter(left_regex), Self::Filter(right_regex)) => {
                left_regex.as_str() == right_regex.as_str()
            }
            (Self::Append(left_suffix), Self::Append(right_suffix)) => left_suffix == right_suffix,
            (Self::Prepend(left_prefix), Self::Prepend(right_prefix)) => {
                left_prefix == right_prefix
            }
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
