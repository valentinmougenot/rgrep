use crate::{
    ast::{Ast, ClassItem, RepetitionKind},
    nfa::{AssertKind, CharRange, Nfa, NfaBuilder, State},
};

pub(crate) struct Compiler {
    builder: NfaBuilder,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            builder: NfaBuilder::new(),
        }
    }

    pub fn build(mut self, ast: &Ast) -> Nfa {
        let fragment = self.compile(ast);
        self.builder.patch_state(fragment.exit, State::Match);
        self.builder.build(fragment.entry, fragment.exit)
    }

    fn compile(&mut self, ast: &Ast) -> Fragment {
        match ast {
            Ast::Literal(c) => self.compile_literal(*c),
            Ast::Concat(values) => self.compile_concat(values),
            Ast::Alternation(values) => self.compile_alternation(values),
            Ast::Group(value) => self.compile_group(value),
            Ast::Dot => self.compile_dot(),
            Ast::CharClass { negative, items } => self.compile_char_class(items, *negative),
            Ast::Repetition { kind, inner } => self.compile_repetition(*kind, inner),
            Ast::StartAnchor => self.compile_start_anchor(),
            Ast::EndAnchor => self.compile_end_anchor(),
            Ast::WordBoundary => self.compile_word_boundary(),
            Ast::WordStart => self.compile_word_start(),
            Ast::WordEnd => self.compile_word_end(),
        }
    }

    fn compile_literal(&mut self, c: char) -> Fragment {
        let exit = self.builder.push_state(State::Split(vec![]));
        let entry = self
            .builder
            .push_state(State::Consuming(vec![CharRange::new(c, c)], exit));

        Fragment { entry, exit }
    }

    fn compile_concat(&mut self, values: &[Ast]) -> Fragment {
        if values.is_empty() {
            let exit = self.builder.push_state(State::Split(vec![]));
            let entry = self.builder.push_state(State::Split(vec![exit]));
            return Fragment { entry, exit };
        }

        let first_fragment = self.compile(&values[0]);
        let mut exit_prev = first_fragment.exit;

        for value in values.iter().skip(1) {
            let fragment = self.compile(value);
            self.builder
                .patch_state(exit_prev, State::Split(vec![fragment.entry]));
            exit_prev = fragment.exit;
        }

        Fragment {
            entry: first_fragment.entry,
            exit: exit_prev,
        }
    }

    fn compile_alternation(&mut self, values: &[Ast]) -> Fragment {
        let entry = self.builder.push_state(State::Split(vec![]));
        let exit = self.builder.push_state(State::Split(vec![]));

        let mut split_values = Vec::with_capacity(values.len());
        for value in values {
            let fragment = self.compile(value);
            split_values.push(fragment.entry);
            self.builder
                .patch_state(fragment.exit, State::Split(vec![exit]));
        }

        self.builder.patch_state(entry, State::Split(split_values));
        Fragment { entry, exit }
    }

    fn compile_group(&mut self, value: &Ast) -> Fragment {
        self.compile(value)
    }

    fn compile_dot(&mut self) -> Fragment {
        let exit = self.builder.push_state(State::Split(vec![]));
        let entry = self.builder.push_state(State::Consuming(
            CharRange::complement(&[CharRange::new('\n', '\n')]),
            exit,
        ));

        Fragment { entry, exit }
    }

    fn compile_char_class(&mut self, items: &[ClassItem], negative: bool) -> Fragment {
        let exit = self.builder.push_state(State::Split(vec![]));

        let mut ranges = Vec::with_capacity(items.len());
        for item in items {
            let range = match item {
                ClassItem::Char(c) => CharRange::new(*c, *c),
                ClassItem::Range(c1, c2) => CharRange::new(*c1, *c2),
            };
            ranges.push(range);
        }

        if negative {
            ranges = CharRange::complement(&ranges);
        }

        let entry = self.builder.push_state(State::Consuming(ranges, exit));

        Fragment { entry, exit }
    }

    fn compile_repetition(&mut self, kind: RepetitionKind, value: &Ast) -> Fragment {
        match kind {
            RepetitionKind::ZeroOrOne => self.compile_zero_or_one(value),
            RepetitionKind::ZeroOrMore => self.compile_zero_or_more(value),
            RepetitionKind::OneOrMore => self.compile_one_or_more(value),
        }
    }

    fn compile_zero_or_one(&mut self, value: &Ast) -> Fragment {
        let inner = self.compile(value);
        let exit = self.builder.push_state(State::Split(vec![]));
        let entry = self
            .builder
            .push_state(State::Split(vec![inner.entry, exit]));
        self.builder
            .patch_state(inner.exit, State::Split(vec![exit]));

        Fragment { entry, exit }
    }

    fn compile_zero_or_more(&mut self, value: &Ast) -> Fragment {
        let inner = self.compile(value);
        let exit = self.builder.push_state(State::Split(vec![]));
        let entry = self
            .builder
            .push_state(State::Split(vec![inner.entry, exit]));
        self.builder
            .patch_state(inner.exit, State::Split(vec![inner.entry, exit]));

        Fragment { entry, exit }
    }

    fn compile_one_or_more(&mut self, value: &Ast) -> Fragment {
        let inner = self.compile(value);
        let exit = self.builder.push_state(State::Split(vec![]));
        let entry = self.builder.push_state(State::Split(vec![inner.entry]));
        self.builder
            .patch_state(inner.exit, State::Split(vec![inner.entry, exit]));

        Fragment { entry, exit }
    }

    fn compile_start_anchor(&mut self) -> Fragment {
        let exit = self.builder.push_state(State::Split(vec![]));
        let entry = self
            .builder
            .push_state(State::Assert(AssertKind::Start, exit));

        Fragment { entry, exit }
    }

    fn compile_end_anchor(&mut self) -> Fragment {
        let exit = self.builder.push_state(State::Split(vec![]));
        let entry = self
            .builder
            .push_state(State::Assert(AssertKind::End, exit));

        Fragment { entry, exit }
    }

    fn compile_word_boundary(&mut self) -> Fragment {
        let exit = self.builder.push_state(State::Split(vec![]));
        let entry = self
            .builder
            .push_state(State::Assert(AssertKind::WordBoundary, exit));

        Fragment { entry, exit }
    }

    fn compile_word_start(&mut self) -> Fragment {
        let exit = self.builder.push_state(State::Split(vec![]));
        let entry = self
            .builder
            .push_state(State::Assert(AssertKind::WordStart, exit));

        Fragment { entry, exit }
    }

    fn compile_word_end(&mut self) -> Fragment {
        let exit = self.builder.push_state(State::Split(vec![]));
        let entry = self
            .builder
            .push_state(State::Assert(AssertKind::WordEnd, exit));

        Fragment { entry, exit }
    }
}

struct Fragment {
    entry: usize,
    exit: usize,
}
