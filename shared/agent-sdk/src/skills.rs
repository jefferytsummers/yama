//! Skill system for agent capabilities.
//!
//! Skills are bundles of system prompt additions, tool configurations,
//! and behavioral guidelines loaded from SKILL.md files.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// A skill that can be loaded into an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Skill name.
    pub name: String,
    /// Skill description.
    pub description: String,
    /// System prompt additions.
    pub system_prompt: String,
    /// Tool configurations.
    pub tools: Vec<SkillTool>,
    /// Behavioral guidelines.
    pub guidelines: Vec<String>,
    /// Example interactions.
    pub examples: Vec<SkillExample>,
}

/// A tool defined by a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTool {
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: String,
    /// When to use this tool.
    pub when_to_use: String,
}

/// An example interaction for a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExample {
    /// User input.
    pub user: String,
    /// Expected assistant response.
    pub assistant: String,
}

/// Loader for skills from SKILL.md files.
pub struct SkillLoader {
    /// Loaded skills by name.
    skills: HashMap<String, Skill>,
}

impl SkillLoader {
    /// Create a new skill loader.
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Load a skill from a SKILL.md file.
    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<&Skill> {
        let path = path.as_ref();
        let skill_path = if path.is_dir() {
            path.join("SKILL.md")
        } else {
            path.to_path_buf()
        };

        info!("Loading skill from {:?}", skill_path);

        let content = std::fs::read_to_string(&skill_path)
            .with_context(|| format!("Failed to read skill file: {:?}", skill_path))?;

        let skill = Self::parse_skill_md(&content)?;
        let name = skill.name.clone();

        debug!("Loaded skill: {}", name);
        self.skills.insert(name.clone(), skill);

        Ok(self.skills.get(&name).unwrap())
    }

    /// Parse a SKILL.md file into a Skill.
    fn parse_skill_md(content: &str) -> Result<Skill> {
        let mut name = String::new();
        let mut description = String::new();
        let mut system_prompt = String::new();
        let mut tools = Vec::new();
        let mut guidelines = Vec::new();
        let mut examples = Vec::new();

        let mut current_section = "";
        let mut current_content = String::new();

        for line in content.lines() {
            if line.starts_with("# ") {
                // Main title - skill name
                name = line.trim_start_matches("# ").trim().to_string();
            } else if line.starts_with("## ") {
                // Process previous section
                Self::process_section(
                    current_section,
                    &current_content,
                    &mut description,
                    &mut system_prompt,
                    &mut tools,
                    &mut guidelines,
                    &mut examples,
                );
                current_content.clear();

                // Start new section
                current_section = line.trim_start_matches("## ").trim();
            } else {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        // Process final section
        Self::process_section(
            current_section,
            &current_content,
            &mut description,
            &mut system_prompt,
            &mut tools,
            &mut guidelines,
            &mut examples,
        );

        Ok(Skill {
            name,
            description,
            system_prompt,
            tools,
            guidelines,
            examples,
        })
    }

    /// Process a section of the SKILL.md file.
    #[allow(clippy::too_many_arguments)]
    fn process_section(
        section: &str,
        content: &str,
        description: &mut String,
        system_prompt: &mut String,
        tools: &mut Vec<SkillTool>,
        guidelines: &mut Vec<String>,
        examples: &mut Vec<SkillExample>,
    ) {
        let content = content.trim();
        if content.is_empty() {
            return;
        }

        match section.to_lowercase().as_str() {
            "description" => {
                *description = content.to_string();
            }
            "system prompt" | "system_prompt" => {
                *system_prompt = content.to_string();
            }
            "tools" => {
                // Parse tool definitions
                for tool_block in content.split("\n### ").skip(1) {
                    let lines: Vec<&str> = tool_block.lines().collect();
                    if let Some(name) = lines.first() {
                        let name = name.trim().to_string();
                        let desc = lines.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
                        let when = lines.get(2..).map(|s| s.join(" ").trim().to_string()).unwrap_or_default();

                        tools.push(SkillTool {
                            name,
                            description: desc,
                            when_to_use: when,
                        });
                    }
                }
            }
            "guidelines" => {
                // Parse bullet points
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("- ") || line.starts_with("* ") {
                        guidelines.push(line[2..].trim().to_string());
                    }
                }
            }
            "examples" => {
                // Parse example interactions
                let mut current_user = String::new();
                let mut current_assistant = String::new();
                let mut in_user = false;
                let mut in_assistant = false;

                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("User:") || line.starts_with("**User:**") {
                        if !current_user.is_empty() {
                            examples.push(SkillExample {
                                user: current_user.clone(),
                                assistant: current_assistant.clone(),
                            });
                            current_assistant.clear();
                        }
                        current_user = line
                            .trim_start_matches("User:")
                            .trim_start_matches("**User:**")
                            .trim()
                            .to_string();
                        in_user = true;
                        in_assistant = false;
                    } else if line.starts_with("Assistant:") || line.starts_with("**Assistant:**") {
                        current_assistant = line
                            .trim_start_matches("Assistant:")
                            .trim_start_matches("**Assistant:**")
                            .trim()
                            .to_string();
                        in_user = false;
                        in_assistant = true;
                    } else if in_user {
                        current_user.push(' ');
                        current_user.push_str(line);
                    } else if in_assistant {
                        current_assistant.push(' ');
                        current_assistant.push_str(line);
                    }
                }

                if !current_user.is_empty() {
                    examples.push(SkillExample {
                        user: current_user,
                        assistant: current_assistant,
                    });
                }
            }
            _ => {
                // Unknown section, add to system prompt
                if !system_prompt.is_empty() {
                    system_prompt.push_str("\n\n");
                }
                system_prompt.push_str(&format!("## {section}\n\n{content}"));
            }
        }
    }

    /// Get a loaded skill by name.
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// Get all loaded skills.
    pub fn all(&self) -> impl Iterator<Item = &Skill> {
        self.skills.values()
    }

    /// Get the number of loaded skills.
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// Check if no skills are loaded.
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Build a combined system prompt from all loaded skills.
    pub fn build_system_prompt(&self, base_prompt: &str) -> String {
        let mut prompt = base_prompt.to_string();

        for skill in self.skills.values() {
            if !skill.system_prompt.is_empty() {
                prompt.push_str("\n\n");
                prompt.push_str(&format!("## {} Skill\n\n", skill.name));
                prompt.push_str(&skill.system_prompt);
            }

            if !skill.guidelines.is_empty() {
                prompt.push_str("\n\nGuidelines:\n");
                for guideline in &skill.guidelines {
                    prompt.push_str(&format!("- {guideline}\n"));
                }
            }
        }

        prompt
    }
}

impl Default for SkillLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_skill_md() {
        let content = r#"
# Video Analysis

## Description
Analyze video streams and describe what you see.

## System Prompt
You are a video analysis assistant. When provided with video frames,
describe the scene, identify objects, and answer questions.

## Guidelines
- Be specific about object locations
- Note any motion or changes
- Describe lighting and environment

## Tools
### get_current_frame
Get the latest video frame from a source.
Use this when the user asks about what's currently visible.

### get_detections
Get AI detection metadata for a frame.
Use this to get bounding boxes and confidence scores.

## Examples
**User:** What do you see in the video?
**Assistant:** I can see a busy street scene with several pedestrians...
"#;

        let skill = SkillLoader::parse_skill_md(content).expect("should parse");

        assert_eq!(skill.name, "Video Analysis");
        assert!(!skill.description.is_empty());
        assert!(!skill.system_prompt.is_empty());
        assert_eq!(skill.guidelines.len(), 3);
        assert_eq!(skill.tools.len(), 2);
        assert_eq!(skill.examples.len(), 1);
    }
}
