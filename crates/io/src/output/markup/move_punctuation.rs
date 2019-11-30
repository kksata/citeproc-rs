use super::InlineElement;
use crate::output::micro_html::MicroNode;

fn is_punc(c: char) -> bool {
    c == '.' || c == ',' || c == '!' || c == '?'
}

// Basically, affixes go outside Quoted elements. So we can just look for text elements that come
// right after quoted ones.
pub fn move_punctuation(slice: &mut [InlineElement]) {

    if slice.len() >= 2 {
        // Basically windows(2)/peekable() iteration, but &mut.
        let len = slice.len();
        for i in 0..len - 1 {
            if let Some((first, rest)) = (&mut slice[i..]).split_first_mut() {
                let next = rest
                    .first_mut()
                    .expect("only iterated to len-1, so infallible");

                // Quoted elements are less common, so search for it first
                let insertion_point = if let Some(x) = find_right_quote(first) {
                    x
                } else {
                    continue;
                };

                fn find_string_micro(m: &mut MicroNode) -> Option<&mut String> {
                    match m {
                        MicroNode::Text(string) => Some(string),
                        MicroNode::NoCase(nodes) | MicroNode::Formatted(nodes, _) => {
                            nodes.first_mut().and_then(find_string_micro)
                        }
                        _ => None,
                    }
                }

                // Must be followed by some text
                let string = match next {
                    InlineElement::Text(ref mut string) => string,
                    InlineElement::Micro(ref mut micros) => {
                        if let Some(string) = micros.first_mut().and_then(find_string_micro) {
                            string
                        } else {
                            continue;
                        }
                    }
                    _ => continue,
                };

                // That text must be is_punc
                if !string.chars().nth(0).map_or(false, is_punc) {
                    continue;
                }

                // O(n), but n tends to be 2, like with ", " so this is ok
                let c = string.remove(0);

                // "Something?," is bad, so stop at removing it from the ", "
                if insertion_point.ends_with_punctuation() {
                    continue;
                }

                let mut s = String::new();
                s.push(c);

                match insertion_point {
                    RightQuoteInsertionPoint::Inline(inlines) => {
                        inlines.push(InlineElement::Text(s));
                    }
                    RightQuoteInsertionPoint::Micro(children) => {
                        children.push(MicroNode::Text(s));
                    }
                }
            }
        }
    } else {
        // recurse manually over the 0 or 1 items in it, and their children
        for inl in slice.iter_mut() {
            match inl {
                InlineElement::Quoted { inlines, .. }
                | InlineElement::Div(_, inlines)
                | InlineElement::Formatted(inlines, _) => move_punctuation(inlines),
                _ => {}
            }
        }
    }
}

// The following functions search inwards, right-leaning, through formatting and as many quotes as possible.
//
// We're trying to find | in these, from $:
//
// "Quoted|"$
// <i>"Quoted|"</i>$
// <i>"'Quoted|'"</i>$
//
// Additionally, we are trying to avoid doubling up. If there's already punctuation right before |,
// don't actually insert anything.

/// "Insertion" == push to one of these vectors.
enum RightQuoteInsertionPoint<'a> {
    Inline(&'a mut Vec<InlineElement>),
    Micro(&'a mut Vec<MicroNode>),
}

fn find_right_quote_micro<'b>(micro: &'b mut MicroNode) -> Option<RightQuoteInsertionPoint<'b>> {
    match micro {
        MicroNode::Quoted {
            localized,
            children,
            ..
        } => {
            if localized.punctuation_in_quote {
                // prefer to dive deeper, and catch "'inner quotes,'" too.

                // This is a limitation of NLL borrowck analysis at the moment, but will be
                // solved with Polonius: https://users.rust-lang.org/t/solved-borrow-doesnt-drop-returning-this-value-requires-that/24182
                //
                // The unsafe is casting a vec to itself; it's safe.
                //
                // let deeper = children.last_mut().and_then(find_right_quote_micro);
                // if deeper.is_some() {
                //     return deeper;
                // }

                if !children.is_empty() {
                    let len = children.len();
                    let last_mut =
                        unsafe { &mut (*((children) as *mut Vec<MicroNode>))[len - 1] };
                    let deeper = find_right_quote_micro(last_mut);
                    if deeper.is_some() {
                        return deeper;
                    }
                }

                Some(RightQuoteInsertionPoint::Micro(children))
            } else {
                None
            }
        }
        // Dive into formatted bits
        MicroNode::NoCase(nodes) | MicroNode::Formatted(nodes, _) => {
            nodes.last_mut().and_then(find_right_quote_micro)
        }
        _ => None,
    }
}

fn find_right_quote<'a>(el: &'a mut InlineElement) -> Option<RightQuoteInsertionPoint<'a>> {
    match el {
        InlineElement::Quoted {
            localized, inlines, ..
        } => {
            if localized.punctuation_in_quote {
                // prefer to dive deeper, and catch "'inner quotes,'" too.

                // See above re unsafe
                if !inlines.is_empty() {
                    let len = inlines.len();
                    let last_mut =
                        unsafe { &mut (*((inlines) as *mut Vec<InlineElement>))[len - 1] };
                    let deeper = find_right_quote(last_mut);
                    if deeper.is_some() {
                        return deeper;
                    }
                }
                Some(RightQuoteInsertionPoint::Inline(inlines))
            } else {
                None
            }
        }
        InlineElement::Micro(micros) => micros.last_mut().and_then(find_right_quote_micro),
        InlineElement::Div(_, inlines) | InlineElement::Formatted(inlines, _) => {
            inlines.last_mut().and_then(find_right_quote)
        }
        _ => None,
    }
}

impl RightQuoteInsertionPoint<'_> {
    fn ends_with_punctuation(&self) -> bool {
        match self {
            RightQuoteInsertionPoint::Inline(inlines) => {
                inlines.last().map_or(false, ends_with_punctuation)
            }
            RightQuoteInsertionPoint::Micro(micros) => {
                micros.last().map_or(false, ends_with_punctuation_micro)
            }
        }
    }
}

fn ends_with_punctuation(i: &InlineElement) -> bool {
    match i {
        InlineElement::Micro(micros) => micros.last().map_or(false, ends_with_punctuation_micro),
        InlineElement::Quoted { inlines, .. } |
        InlineElement::Div(_, inlines) |
        InlineElement::Formatted(inlines, _) => {
            inlines.last().map_or(false, ends_with_punctuation)
        }
        InlineElement::Text(string) => string.chars().last().map_or(false, is_punc),
        _ => false,
    }
}

fn ends_with_punctuation_micro(i: &MicroNode) -> bool {
    match i {
        MicroNode::Quoted { children, .. } |
        MicroNode::NoCase(children) |
        MicroNode::Formatted(children, _) => {
            children.last().map_or(false, ends_with_punctuation_micro)
        }
        MicroNode::Text(string) => string.chars().last().map_or(false, is_punc),
    }
}
