use id_arena::{Arena, Id};

type AsciiString = Vec<u8>;
type AsciiStringRef<'a> = &'a [u8];

type NFAStateId = Id<NFAState>;
type VisitedNodes = Vec<NFAStateId>;

struct NFAState {
    is_end: bool,
    char_transition: Vec<Option<NFAStateId>>,
    epsilon_transition: Vec<NFAStateId>,
}

impl NFAState {
    fn new() -> Self {
        NFAState {
            is_end: false,
            char_transition: vec![None; 256],
            epsilon_transition: vec![],
        }
    }

    fn add_epsilon(&mut self, to: NFAStateId) {
        self.epsilon_transition.push(to)
    }
}

struct NFAFragment {
    start: NFAStateId,
    out: NFAStateId,
}

impl NFAFragment {
    fn new(arena: &mut Arena<NFAState>) -> Self {
        let start = arena.alloc(NFAState::new());
        let out = arena.alloc(NFAState::new());

        arena[start].is_end = false;
        arena[out].is_end = true;

        NFAFragment { start, out }
    }
}

fn insert_concat_operator(regexp_bytes: AsciiStringRef) -> AsciiString {
    let n = regexp_bytes.len();
    let mut result = Vec::with_capacity(n + n + 1);

    for i in 0..n {
        let at = regexp_bytes[i];
        result.push(at);

        if at == ('(' as u8) || at == ('|' as u8) {
            continue;
        }

        if i < n - 1 {
            let next = regexp_bytes[i + 1];

            if next == ('|' as u8)
                || next == ('+' as u8)
                || next == ('*' as u8)
                || next == (')' as u8)
                || next == ('?' as u8)
            {
                continue;
            }

            result.push('.' as u8);
        }
    }

    result
}

fn operator_precedence(c: u8) -> u8 {
    match c as char {
        '.' => 0,
        '|' => 1,
        '+' => 2,
        '?' => 2,
        '*' => 2,
        _ => 0,
    }
}

fn is_operator(c: u8) -> bool {
    (c == '|' as u8) || (c == '+' as u8) || (c == '.' as u8) || (c == '*' as u8) || (c == '?' as u8)
}

fn regexp_to_postfix(regexp: AsciiStringRef) -> AsciiString {
    let n = regexp.len();
    let mut result: Vec<u8> = Vec::with_capacity(n + n + 1);
    let mut stack: Vec<u8> = Vec::with_capacity(n + n + 1);

    for i in 0..n {
        let token: u8 = regexp[i];

        if is_operator(token) {
            let precedence = operator_precedence(token);

            while let Some(c) = stack.last() {
                if *c != '(' as u8 && operator_precedence(stack[0]) >= precedence {
                    result.push(*c);
                    stack.pop();
                } else {
                    break;
                }
            }

            stack.push(token);
        } else if token == '(' as u8 {
            stack.push(token);
        } else if token == ')' as u8 {
            while let Some(c) = stack.last() {
                if *c != '(' as u8 {
                    result.push(*c);
                    stack.pop();
                } else {
                    break;
                }
            }
            stack.pop();
        } else {
            result.push(token);
        }
    }

    while let Some(c) = stack.last() {
        result.push(*c);
        stack.pop();
    }

    result
}

fn postfix_to_nfa(arena: &mut Arena<NFAState>, postfix_regexp: AsciiStringRef) -> NFAFragment {
    let n = postfix_regexp.len();
    let mut stack: Vec<NFAFragment> = Vec::with_capacity(n);

    for i in 0..n {
        let at = postfix_regexp[i];
        match at as char {
            '.' => {
                let right = stack.pop().unwrap();
                let left = stack.pop().unwrap();

                arena[left.out].is_end = false;
                arena[left.out].add_epsilon(right.start);

                let mut frag = NFAFragment::new(arena);
                frag.start = left.start;
                frag.out = right.out;
                stack.push(frag);
            }
            '|' => {
                let right = stack.pop().unwrap();
                let left = stack.pop().unwrap();

                let frag = NFAFragment::new(arena);
                arena[frag.start].add_epsilon(right.start);
                arena[frag.start].add_epsilon(left.start);

                arena[left.out].is_end = false;
                arena[left.out].add_epsilon(frag.out);

                arena[right.out].is_end = false;
                arena[right.out].add_epsilon(frag.out);

                stack.push(frag);
            }
            '?' => {
                let op = stack.pop().unwrap();

                let frag = NFAFragment::new(arena);
                arena[frag.start].add_epsilon(frag.out);
                arena[frag.start].add_epsilon(op.start);
                arena[op.out].add_epsilon(frag.out);
                arena[op.out].is_end = false;

                stack.push(frag);
            }
            '+' => {
                let op = stack.pop().unwrap();

                let frag = NFAFragment::new(arena);
                arena[frag.start].add_epsilon(op.start);
                arena[op.out].add_epsilon(op.start);
                arena[op.out].add_epsilon(frag.out);
                arena[op.out].is_end = false;

                stack.push(frag);
            }
            '*' => {
                let op = stack.pop().unwrap();

                let frag = NFAFragment::new(arena);
                arena[frag.start].add_epsilon(op.start);
                arena[frag.start].add_epsilon(frag.out);
                arena[op.out].add_epsilon(op.start);
                arena[op.out].add_epsilon(frag.out);
                arena[op.out].is_end = false;

                stack.push(frag);
            }
            _ => {
                let frag = NFAFragment::new(arena);
                arena[frag.start].char_transition[at as usize] = Some(frag.out);
                stack.push(frag);
            }
        }
    }

    let result = stack.pop().unwrap();
    result
}

fn already_visited(v: &VisitedNodes, node: NFAStateId) -> bool {
    for e in v {
        if *e == node {
            return true;
        }
    }

    false
}

fn dfs(
    arena: &Arena<NFAState>,
    root: NFAStateId,
    word: AsciiStringRef,
    matched_num: usize,
    visited: &mut VisitedNodes,
) -> bool {
    if already_visited(visited, root) {
        return false;
    }

    visited.push(root);
    if word.len() == matched_num {
        if arena[root].is_end {
            return true;
        }

        for v in &arena[root].epsilon_transition {
            if dfs(arena, *v, word, matched_num, visited) {
                return true;
            }
        }
    } else {
        if let Some(transition) = arena[root].char_transition[word[matched_num] as usize % 128] {
            let mut visited = VisitedNodes::with_capacity(256);
            if dfs(arena, transition, word, matched_num + 1, &mut visited) {
                return true;
            }
        } else {
            for v in &arena[root].epsilon_transition {
                if dfs(arena, *v, word, matched_num, visited) {
                    return true;
                }
            }
        }
    }

    false
}

fn is_match(arena: &Arena<NFAState>, nfa: NFAFragment, search: AsciiStringRef) -> bool {
    let mut visited = vec![];
    dfs(&arena, nfa.start, search, 0, &mut visited)
}

fn re(regexp: AsciiStringRef) -> (NFAFragment, Arena<NFAState>) {
    let mut arena: Arena<NFAState> = Arena::new();
    let concatted = insert_concat_operator(regexp);
    let postfix = regexp_to_postfix(&concatted);
    let result = postfix_to_nfa(&mut arena, &postfix);
    (result, arena)
}

pub fn regex_match(regexp: &str, search: &str) -> bool {
    let (compiled, arena) = re(regexp.as_bytes());
    is_match(&arena, compiled, search.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_match() {
        assert_eq!(true, regex_match("(zz)+", "zz"));
        assert_eq!(true, regex_match("(x|y)*z", "xyxyyyxxxz"));
        assert_eq!(false, regex_match("(x|y)*z+", "xy"));
        assert_eq!(true, regex_match("(x|y)*z+", "xyzzz"));
        assert_eq!(true, regex_match("(1|2|3|4|5|6|7|8|9)+", "1423"));
        assert_eq!(false, regex_match("(1|2|3|4|5|6|7|8|9)+", "123abc"));
        assert_eq!(true, regex_match("a?", ""));
        assert_eq!(true, regex_match("a?", "a"));
        assert_eq!(false, regex_match("a?", "aa"));
        assert_eq!(true, regex_match("hell(a|o)?", "hello"));
        assert_eq!(true, regex_match("(a|b)?", "a"));
    }
}
