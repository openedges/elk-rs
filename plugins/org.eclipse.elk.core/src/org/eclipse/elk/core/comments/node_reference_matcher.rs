use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::org::eclipse::elk::core::comments::cache_key::CacheKey;
use crate::org::eclipse::elk::core::comments::distance_matcher::distance;
use crate::org::eclipse::elk::core::comments::i_bounds_provider::IBoundsProvider;
use crate::org::eclipse::elk::core::comments::i_data_provider::IDataProvider;
use crate::org::eclipse::elk::core::comments::i_matcher::IMatcher;
use crate::org::eclipse::elk::core::util::Pair;

type CommentTextProvider<C> = Rc<dyn Fn(&C) -> String>;
type TargetNameProvider<T> = Rc<dyn Fn(&T) -> String>;

pub struct NodeReferenceMatcher<C, T> {
    comment_text_provider: Option<CommentTextProvider<C>>,
    target_name_provider: Option<TargetNameProvider<T>>,
    bounds_provider: Option<Rc<dyn IBoundsProvider<C, T>>>,
    max_distance: f64,
    fuzzy: bool,
    found_attachments: RefCell<HashMap<usize, Pair<C, T>>>,
}

enum TargetPattern {
    Strict(String),
    Fuzzy(Vec<String>),
}

impl<C: 'static, T: 'static> NodeReferenceMatcher<C, T> {
    pub fn new() -> Self {
        NodeReferenceMatcher {
            comment_text_provider: None,
            target_name_provider: None,
            bounds_provider: None,
            max_distance: -1.0,
            fuzzy: false,
            found_attachments: RefCell::new(HashMap::new()),
        }
    }

    pub fn with_comment_text_provider<F>(&mut self, f: F) -> &mut Self
    where
        F: Fn(&C) -> String + 'static,
    {
        self.comment_text_provider = Some(Rc::new(f));
        self
    }

    pub fn with_target_name_provider<F>(&mut self, f: F) -> &mut Self
    where
        F: Fn(&T) -> String + 'static,
    {
        self.target_name_provider = Some(Rc::new(f));
        self
    }

    pub fn with_fuzzy_matching(&mut self) -> &mut Self {
        self.fuzzy = true;
        self
    }

    pub fn with_maximum_attachment_distance(&mut self, distance: f64) -> &mut Self {
        self.max_distance = distance;
        self
    }

    pub fn with_bounds_provider(&mut self, provider: Rc<dyn IBoundsProvider<C, T>>) -> &mut Self {
        self.bounds_provider = Some(provider);
        self
    }

    pub fn get_attachments(&self) -> Vec<Pair<C, T>>
    where
        C: Clone,
        T: Clone,
    {
        self.found_attachments.borrow().values().cloned().collect()
    }

    fn check_configuration(&self) {
        if self.comment_text_provider.is_none() {
            panic!("A comment text function is required.");
        }
        if self.target_name_provider.is_none() {
            panic!("A node name function is required.");
        }
        if self.max_distance >= 0.0 && self.bounds_provider.is_none() {
            panic!("A bounds provider must be installed if a maximum attachment distance is set.");
        }
    }

    fn go_find_matches(
        &self,
        comment_texts: Vec<Pair<C, String>>,
        target_names: Vec<Pair<T, String>>,
    ) where
        C: CacheKey + Clone,
        T: CacheKey + Clone,
    {
        let mut target_patterns = Vec::with_capacity(target_names.len());
        for target_name_pair in target_names {
            let pattern = if self.fuzzy {
                TargetPattern::Fuzzy(
                    fuzzy_segments(&target_name_pair.second)
                        .into_iter()
                        .map(|segment| segment.to_lowercase())
                        .collect(),
                )
            } else {
                TargetPattern::Strict(target_name_pair.second)
            };
            target_patterns.push(Pair::of(target_name_pair.first, pattern));
        }

        for comment_text_pair in comment_texts {
            let mut found_target: Option<T> = None;

            for target_pattern_pair in &target_patterns {
                let matches = match &target_pattern_pair.second {
                    TargetPattern::Strict(name) => strict_match(&comment_text_pair.second, name),
                    TargetPattern::Fuzzy(segments) => {
                        fuzzy_match(&comment_text_pair.second, segments)
                    }
                };
                if matches {
                    if found_target.is_none() {
                        found_target = Some(target_pattern_pair.first.clone());
                    } else {
                        found_target = None;
                        break;
                    }
                }
            }

            if let Some(target) = found_target {
                if self.max_distance < 0.0 {
                    let key = comment_text_pair.first.cache_key();
                    self.found_attachments
                        .borrow_mut()
                        .insert(key, Pair::of(comment_text_pair.first, target));
                } else if let Some(provider) = self.bounds_provider.as_ref() {
                    let comment_bounds = provider.bounds_for_comment(&comment_text_pair.first);
                    let target_bounds = provider.bounds_for_target(&target);
                    if let (Some(comment_bounds), Some(target_bounds)) =
                        (comment_bounds, target_bounds)
                    {
                        if distance(&comment_bounds, &target_bounds) <= self.max_distance {
                            let key = comment_text_pair.first.cache_key();
                            self.found_attachments
                                .borrow_mut()
                                .insert(key, Pair::of(comment_text_pair.first, target));
                        }
                    }
                }
            }
        }
    }
}

impl<C: 'static, T: 'static> Default for NodeReferenceMatcher<C, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C, T> IMatcher<C, T> for NodeReferenceMatcher<C, T>
where
    C: CacheKey + Clone + 'static,
    T: CacheKey + Clone + 'static,
{
    fn preprocess(&self, data_provider: &dyn IDataProvider<C, T>, include_hierarchy: bool) {
        self.check_configuration();

        let comment_text_provider = self
            .comment_text_provider
            .as_ref()
            .expect("A comment text function is required.");
        let target_name_provider = self
            .target_name_provider
            .as_ref()
            .expect("A node name function is required.");

        let mut comment_texts = Vec::new();
        for comment in data_provider.provide_comments() {
            let comment_text = comment_text_provider(&comment);
            if !comment_text.is_empty() {
                comment_texts.push(Pair::of(comment, comment_text));
            }
        }

        let mut target_names = Vec::new();
        for target in data_provider.provide_targets() {
            let target_name = target_name_provider(&target);
            if !target_name.is_empty() {
                target_names.push(Pair::of(target, target_name));
            }
        }

        self.go_find_matches(comment_texts, target_names);

        if include_hierarchy {
            for sub_provider in data_provider.provide_sub_hierarchies() {
                self.preprocess(&*sub_provider, true);
            }
        }
    }

    fn cleanup(&self) {
        self.found_attachments.borrow_mut().clear();
    }

    fn raw(&self, comment: &C, target: &T) -> f64 {
        let key = comment.cache_key();
        if let Some(pair) = self.found_attachments.borrow().get(&key) {
            if pair.second.cache_key() == target.cache_key() {
                1.0
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    fn normalized(&self, comment: &C, target: &T) -> f64 {
        self.raw(comment, target)
    }
}

fn strict_match(text: &str, target: &str) -> bool {
    if target.is_empty() {
        return false;
    }
    for (start, _) in text.match_indices(target) {
        let end = start + target.len();
        if is_word_boundary(text, start, end) {
            return true;
        }
    }
    false
}

fn fuzzy_match(text: &str, segments_lower: &[String]) -> bool {
    if segments_lower.is_empty() {
        return false;
    }
    let text_lower = text.to_lowercase();
    fuzzy_match_lower(&text_lower, segments_lower)
}

fn fuzzy_match_lower(text: &str, segments_lower: &[String]) -> bool {
    let first = &segments_lower[0];
    if first.is_empty() {
        return false;
    }

    for (start, _) in text.match_indices(first) {
        let mut idx = start + first.len();
        let mut ok = true;
        for segment in segments_lower.iter().skip(1) {
            idx = skip_whitespace(text, idx);
            if !text[idx..].starts_with(segment) {
                ok = false;
                break;
            }
            idx += segment.len();
        }
        if ok && is_word_boundary(text, start, idx) {
            return true;
        }
    }
    false
}

fn fuzzy_segments(target_name: &str) -> Vec<String> {
    let trimmed_target_name = target_name.trim();
    let chars: Vec<char> = trimmed_target_name.chars().collect();
    let mut segments = Vec::new();
    let mut current_segment = String::new();

    for i in 0..chars.len() {
        let curr_c = chars[i];
        if curr_c.is_uppercase() {
            if i > 0 && chars[i - 1].is_lowercase() {
                if !current_segment.is_empty() {
                    segments.push(current_segment);
                }
                current_segment = String::new();
            }
            current_segment.push(curr_c);
        } else if curr_c.is_whitespace() {
            if i > 0 && !chars[i - 1].is_whitespace() {
                if !current_segment.is_empty() {
                    segments.push(current_segment);
                }
                current_segment = String::new();
            }
        } else {
            current_segment.push(curr_c);
        }
    }

    if !current_segment.is_empty() {
        segments.push(current_segment);
    }

    segments
}

fn skip_whitespace(text: &str, start: usize) -> usize {
    for (idx, ch) in text[start..].char_indices() {
        if !ch.is_whitespace() {
            return start + idx;
        }
    }
    text.len()
}

fn is_word_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = if start == 0 {
        None
    } else {
        text[..start].chars().last()
    };
    let after = if end >= text.len() {
        None
    } else {
        text[end..].chars().next()
    };
    !is_word_char(before) && !is_word_char(after)
}

fn is_word_char(ch: Option<char>) -> bool {
    match ch {
        Some(c) => c.is_alphanumeric() || c == '_',
        None => false,
    }
}
