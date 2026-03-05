use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PostAction {
    #[default]
    Ignore,
    Action(ActionData),
}

impl PostAction {
    pub fn with_module(self, name: &str) -> PostAction {
        match self {
            PostAction::Ignore => PostAction::Ignore,
            PostAction::Action(mut data) => {
                data.set_module(name);
                PostAction::Action(data)
            }
        }
    }

    pub fn join(self, other: PostAction) -> PostAction {
        match (self, other) {
            (PostAction::Ignore, other) => other,
            (this, PostAction::Ignore) => this,

            (PostAction::Action(this_data), PostAction::Action(other_data)) => {
                let data = match (this_data.moderate, other_data.moderate) {
                    (ModAct::None, ModAct::None)
                    | (ModAct::Report, ModAct::Report)
                    | (ModAct::Remove, ModAct::Remove)
                    | (ModAct::Filter, ModAct::Filter) => Self::merge(this_data, other_data),

                    (ModAct::None, ModAct::Report) => other_data,
                    (ModAct::None, ModAct::Remove) => other_data,
                    (ModAct::None, ModAct::Filter) => other_data,

                    (ModAct::Report, ModAct::None) => this_data,
                    (ModAct::Report, ModAct::Remove) => other_data,
                    (ModAct::Report, ModAct::Filter) => other_data,

                    (ModAct::Remove, ModAct::None) => this_data,
                    (ModAct::Remove, ModAct::Report) => this_data,
                    (ModAct::Remove, ModAct::Filter) => other_data,

                    (ModAct::Filter, ModAct::None) => this_data,
                    (ModAct::Filter, ModAct::Report) => this_data,
                    (ModAct::Filter, ModAct::Remove) => this_data,
                };

                PostAction::Action(data)
            }
        }
    }

    fn merge(this: ActionData, other: ActionData) -> ActionData {
        let module = if this.module.len() == 0 {
            other.module
        } else {
            this.module + "," + other.module.as_str()
        };

        let analyser = match (this.analyser, other.analyser) {
            (None, None) => None,
            (None, Some(t)) | (Some(t), None) => Some(t),
            (Some(mut l), Some(r)) => {
                l.push(',');
                l.push_str(&r);
                Some(l)
            }
        };

        let reply = match (this.reply, other.reply) {
            (None, None) => None,
            (None, Some(reply)) | (Some(reply), None) => Some(reply),
            (Some(mut this), Some(other)) => {
                this.text.push_str("\n------\n");
                this.text.push_str(&other.text);

                this.distinguish = this.distinguish | other.distinguish;

                Some(this)
            }
        };

        ActionData {
            module,
            analyser,
            reply,
            moderate: this.moderate,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActionData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analyser: Option<String>,
    pub module: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply: Option<PostReply>,
    pub moderate: ModAct,
}

impl ActionData {
    pub fn new() -> Self {
        Self {
            analyser: None,
            module: String::new(),
            reply: None,
            moderate: ModAct::None,
        }
    }

    fn set_module(&mut self, name: &str) -> &mut Self {
        self.module = name.to_string();
        self
    }

    pub fn module(mut self, name: &str) -> Self {
        self.set_module(name);
        self
    }

    pub fn analyser(mut self, name: &str) -> Self {
        self.analyser = Some(name.to_string());
        self
    }

    pub fn reply(mut self, text: String, distinguish: bool) -> Self {
        self.set_reply(text, distinguish);
        self
    }

    pub fn set_reply(&mut self, text: String, distinguish: bool) -> &mut Self {
        self.reply = Some(PostReply { text, distinguish });
        self
    }

    pub fn set_report(&mut self) -> &mut Self {
        self.moderate = ModAct::Report;
        self
    }

    pub fn report(mut self) -> Self {
        self.set_report();
        self
    }

    pub fn set_remove(&mut self) -> &mut Self {
        self.moderate = ModAct::Remove;
        self
    }

    pub fn remove(mut self) -> Self {
        self.set_remove();
        self
    }

    pub fn set_filter(&mut self) -> &mut Self {
        self.moderate = ModAct::Filter;
        self
    }

    pub fn filter(mut self) -> Self {
        self.set_filter();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PostReply {
    pub text: String,
    pub distinguish: bool,
}

#[derive(Debug, Clone, PartialEq, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModAct {
    /// Take no moderation decisions
    None,
    /// Report the post
    Report,
    /// Remove the post
    Remove,
    /// Remove the post and send a message to the subreddit's modmail
    Filter,
}

#[cfg(test)]
mod tests {
    use super::{ActionData, PostAction};

    #[test]
    pub fn test_ignore_overriden() {
        let first = PostAction::Ignore;
        let second = PostAction::Action(ActionData::new());

        let result = first.clone().join(second.clone());
        assert_eq!(result, second);

        let result = second.clone().join(first.clone());
        assert_eq!(result, second);
    }

    #[test]
    pub fn test_action_remove_takes_precedence() {
        let first = PostAction::Action(ActionData::new().analyser("first").remove());
        let second = PostAction::Action(
            ActionData::new()
                .analyser("second")
                .reply("second text".into(), true),
        );

        let result = first.clone().join(second.clone());
        assert_eq!(result, first);

        let result = second.clone().join(first.clone());
        assert_eq!(result, first);
    }

    #[test]
    pub fn test_merges_equal() {
        let first = PostAction::Action(
            ActionData::new()
                .analyser("first")
                .module("group")
                .remove()
                .reply("first text".into(), false),
        );

        let second = PostAction::Action(
            ActionData::new()
                .analyser("second")
                .module("parent")
                .remove()
                .reply("second text".into(), true),
        );

        let expected = PostAction::Action(
            ActionData::new()
                .analyser("first,second")
                .module("group,parent")
                .remove()
                .reply("first text\n------\nsecond text".into(), true),
        );

        let result = first.clone().join(second.clone());
        assert_eq!(result, expected);

        let expected = PostAction::Action(
            ActionData::new()
                .analyser("second,first")
                .module("parent,group")
                .remove()
                .reply("second text\n------\nfirst text".into(), true),
        );

        let result = second.join(first);
        assert_eq!(result, expected);
    }
}
