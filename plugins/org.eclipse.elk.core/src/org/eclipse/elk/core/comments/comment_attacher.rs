use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use crate::org::eclipse::elk::core::comments::aggregated_match_decider::AggregatedMatchDecider;
use crate::org::eclipse::elk::core::comments::i_bounds_provider::IBoundsProvider;
use crate::org::eclipse::elk::core::comments::i_data_provider::IDataProvider;
use crate::org::eclipse::elk::core::comments::i_decider::{IDecider, NormalizedHeuristics};
use crate::org::eclipse::elk::core::comments::i_explicit_attachment_provider::IExplicitAttachmentProvider;
use crate::org::eclipse::elk::core::comments::i_filter::IFilter;
use crate::org::eclipse::elk::core::comments::i_matcher::IMatcher;
use crate::org::eclipse::elk::core::util::Pair;

pub struct CommentAttacher<C, T> {
    include_hierarchy: bool,
    explicit_attachments_disable_heuristics: bool,
    explicit_attachment_provider: Rc<dyn IExplicitAttachmentProvider<C, T>>,
    bounds_provider: Option<Rc<dyn IBoundsProvider<C, T>>>,
    filters: Vec<Rc<dyn IFilter<C, T>>>,
    matchers: Vec<Rc<dyn IMatcher<C, T>>>,
    decider: Rc<dyn IDecider<T>>,
}

impl<C, T> CommentAttacher<C, T>
where
    C: Clone + 'static,
    T: Clone + 'static,
{
    pub fn new() -> Self {
        CommentAttacher {
            include_hierarchy: true,
            explicit_attachments_disable_heuristics: true,
            explicit_attachment_provider: Rc::new(NoExplicitAttachmentProvider),
            bounds_provider: None,
            filters: Vec::new(),
            matchers: Vec::new(),
            decider: Rc::new(AggregatedMatchDecider::new()),
        }
    }

    pub fn limit_to_current_hierarchy_level(&mut self, limit: bool) -> &mut Self {
        self.include_hierarchy = !limit;
        self
    }

    pub fn keep_heuristics_enabled_with_explicit_attachments(
        &mut self,
        keep_enabled: bool,
    ) -> &mut Self {
        self.explicit_attachments_disable_heuristics = !keep_enabled;
        self
    }

    pub fn with_explicit_attachment_provider(
        &mut self,
        provider: Option<Rc<dyn IExplicitAttachmentProvider<C, T>>>,
    ) -> &mut Self {
        self.explicit_attachment_provider =
            provider.unwrap_or_else(|| Rc::new(NoExplicitAttachmentProvider));
        self
    }

    pub fn with_bounds_provider(&mut self, provider: Rc<dyn IBoundsProvider<C, T>>) -> &mut Self {
        self.bounds_provider = Some(provider);
        self
    }

    pub fn add_filter(&mut self, filter: Rc<dyn IFilter<C, T>>) -> &mut Self {
        self.filters.push(filter);
        self
    }

    pub fn add_matcher(&mut self, matcher: Rc<dyn IMatcher<C, T>>) -> &mut Self {
        self.matchers.push(matcher);
        self
    }

    pub fn with_attachment_decider(&mut self, decider: Rc<dyn IDecider<T>>) -> &mut Self {
        self.decider = decider;
        self
    }

    pub fn attach_comments(&self, data_provider: &dyn IDataProvider<C, T>) {
        self.preprocess(data_provider);

        let mut explicit_attachments: Vec<Pair<C, T>> = Vec::new();
        let mut heuristic_attachments: Vec<Pair<C, T>> = Vec::new();

        self.process_provider(
            data_provider,
            &mut explicit_attachments,
            &mut heuristic_attachments,
        );

        if self.include_hierarchy {
            let mut queue: VecDeque<Rc<dyn IDataProvider<C, T>>> = VecDeque::new();
            queue.extend(data_provider.provide_sub_hierarchies());

            while let Some(provider) = queue.pop_front() {
                self.process_provider(
                    &*provider,
                    &mut explicit_attachments,
                    &mut heuristic_attachments,
                );
                queue.extend(provider.provide_sub_hierarchies());
            }
        }

        self.cleanup();

        self.edgeify_found_attachments(
            data_provider,
            &explicit_attachments,
            &heuristic_attachments,
        );
    }

    fn preprocess(&self, data_provider: &dyn IDataProvider<C, T>) {
        self.explicit_attachment_provider
            .preprocess(data_provider, self.include_hierarchy);
        for filter in &self.filters {
            filter.preprocess(data_provider, self.include_hierarchy);
        }
        for matcher in &self.matchers {
            matcher.preprocess(data_provider, self.include_hierarchy);
        }
        if let Some(bounds_provider) = self.bounds_provider.as_ref() {
            bounds_provider.preprocess(data_provider, self.include_hierarchy);
        }
    }

    fn process_provider(
        &self,
        data_provider: &dyn IDataProvider<C, T>,
        explicit_attachments: &mut Vec<Pair<C, T>>,
        heuristic_attachments: &mut Vec<Pair<C, T>>,
    ) {
        for comment in data_provider.provide_comments() {
            let explicit_attachment = self
                .explicit_attachment_provider
                .find_explicit_attachment(&comment);

            if let Some(target) = explicit_attachment {
                explicit_attachments.push(Pair::of(comment, target));
            } else if (explicit_attachments.is_empty()
                || !self.explicit_attachments_disable_heuristics)
                && self.is_eligible_for_heuristic_attachment(&comment)
            {
                if let Some(target) = self.find_match(data_provider, &comment) {
                    heuristic_attachments.push(Pair::of(comment, target));
                }
            }
        }
    }

    fn is_eligible_for_heuristic_attachment(&self, comment: &C) -> bool {
        self.filters
            .iter()
            .all(|filter| filter.eligible_for_attachment(comment))
    }

    fn find_match(&self, data_provider: &dyn IDataProvider<C, T>, comment: &C) -> Option<T> {
        if self.matchers.is_empty() {
            return None;
        }

        let candidates = data_provider.provide_targets_for(comment);
        if candidates.is_empty() {
            return None;
        }

        let mut results = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let mut values: HashMap<std::any::TypeId, f64> = HashMap::new();
            for matcher in &self.matchers {
                values.insert(
                    matcher.matcher_id(),
                    matcher.normalized(comment, &candidate),
                );
            }
            results.push(NormalizedHeuristics {
                target: candidate,
                values,
            });
        }

        self.decider.make_attachment_decision(&results)
    }

    fn edgeify_found_attachments(
        &self,
        data_provider: &dyn IDataProvider<C, T>,
        explicit_attachments: &[Pair<C, T>],
        heuristic_attachments: &[Pair<C, T>],
    ) {
        for attachment in explicit_attachments {
            data_provider.attach(attachment.first(), attachment.second());
        }

        if explicit_attachments.is_empty() || !self.explicit_attachments_disable_heuristics {
            for attachment in heuristic_attachments {
                data_provider.attach(attachment.first(), attachment.second());
            }
        }
    }

    fn cleanup(&self) {
        self.explicit_attachment_provider.cleanup();
        for filter in &self.filters {
            filter.cleanup();
        }
        for matcher in &self.matchers {
            matcher.cleanup();
        }
        if let Some(bounds_provider) = self.bounds_provider.as_ref() {
            bounds_provider.cleanup();
        }
    }
}

impl<C, T> Default for CommentAttacher<C, T>
where
    C: Clone + 'static,
    T: Clone + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

struct NoExplicitAttachmentProvider;

impl<C, T> IExplicitAttachmentProvider<C, T> for NoExplicitAttachmentProvider {
    fn find_explicit_attachment(&self, _comment: &C) -> Option<T> {
        None
    }
}
