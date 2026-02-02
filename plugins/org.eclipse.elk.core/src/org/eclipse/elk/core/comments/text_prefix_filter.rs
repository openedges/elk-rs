use std::rc::Rc;

use crate::org::eclipse::elk::core::comments::i_filter::IFilter;

type CommentTextProvider<C> = Rc<dyn Fn(&C) -> String>;

pub struct TextPrefixFilter<C, T> {
    comment_text_provider: Option<CommentTextProvider<C>>,
    prefixes: Vec<String>,
    reject_comment_on_prefix_match: bool,
    case_sensitive: bool,
    _phantom: std::marker::PhantomData<T>,
}

impl<C, T> TextPrefixFilter<C, T> {
    pub fn new() -> Self {
        TextPrefixFilter {
            comment_text_provider: None,
            prefixes: Vec::new(),
            reject_comment_on_prefix_match: true,
            case_sensitive: false,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_comment_text_provider<F>(&mut self, f: F) -> &mut Self
    where
        F: Fn(&C) -> String + 'static,
    {
        self.comment_text_provider = Some(Rc::new(f));
        self
    }

    pub fn with_prefix_match_required_for_eligibility(&mut self) -> &mut Self {
        self.reject_comment_on_prefix_match = false;
        self
    }

    pub fn with_case_sensitive_matching(&mut self) -> &mut Self {
        self.case_sensitive = true;
        self
    }

    pub fn add_prefix(&mut self, prefix: &str) -> &mut Self {
        if prefix.is_empty() {
            panic!("Prefix cannot be null or empty. Wouldn't make sense.");
        }
        self.prefixes.push(prefix.to_string());
        self
    }

    fn check_configuration(&self) {
        if self.comment_text_provider.is_none() {
            panic!("A comment text provider is required.");
        }
        if self.prefixes.is_empty() {
            panic!("At least one prefix is required.");
        }
    }
}

impl<C, T> Default for TextPrefixFilter<C, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C, T> IFilter<C, T> for TextPrefixFilter<C, T> {
    fn preprocess(
        &self,
        data_provider: &dyn crate::org::eclipse::elk::core::comments::i_data_provider::IDataProvider<
            C,
            T,
        >,
        include_hierarchy: bool,
    ) {
        let _ = (data_provider, include_hierarchy);
        self.check_configuration();
    }

    fn eligible_for_attachment(&self, comment: &C) -> bool {
        let provider = self
            .comment_text_provider
            .as_ref()
            .expect("A comment text provider is required.");
        let comment_text = provider(comment);
        if !comment_text.is_empty() {
            if self.case_sensitive {
                for prefix in &self.prefixes {
                    if comment_text.starts_with(prefix) {
                        return !self.reject_comment_on_prefix_match;
                    }
                }
            } else {
                let lower_comment = comment_text.to_lowercase();
                for prefix in &self.prefixes {
                    if lower_comment.starts_with(&prefix.to_lowercase()) {
                        return !self.reject_comment_on_prefix_match;
                    }
                }
            }
        }

        self.reject_comment_on_prefix_match
    }
}
