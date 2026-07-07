use std::{cmp::max, io::Cursor, ops::RangeInclusive, slice::Iter};

use ab_glyph::FontVec;

///
/// Font
///
pub struct VkFont {
    glyphs: FontVec,
}

pub(crate) struct CharacterSet<'a> {
    ranges: Iter<'a, RangeInclusive<u32>>,
    chars: Option<RangeInclusive<u32>>,
}

impl<'a> CharacterSet<'a> {
    pub fn new(source: &'a Vec<RangeInclusive<u32>>) -> Self {
        Self {
            ranges: source.iter(),
            chars: None,
        }
    }
}

impl Iterator for CharacterSet<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.chars.is_none() {
                self.chars = self.ranges.next().map(|r| RangeInclusive::clone(r));
            }
            if let Some(it) = self.chars.as_mut() {
                if let Some(ch) = it.next() {
                    if let Some(result) = char::from_u32(ch) {
                        return Some(result);
                    }
                } else {
                    self.chars = None;
                }
            } else {
                break;
            }
        }

        None
    }
}

pub(crate) fn optimize_ranges(source: &Vec<RangeInclusive<u32>>) -> Vec<RangeInclusive<u32>> {
    let mut sorted = Vec::clone(source);
    sorted.sort_by_key(|v| v.start().clone());

    let mut previous = sorted[0].clone();
    let mut result = Vec::with_capacity(source.len());

    for range in sorted.into_iter().skip(1) {
        if range.start() <= &(*previous.end() + 1) {
            previous = (*previous.start()..=max(*previous.end(), *range.end()));
        } else {
            result.push(previous);
            previous = range;
        }
    }
    result.push(previous);
    result
}

#[cfg(test)]
mod tests {
    use crate::font::{CharacterSet, optimize_ranges};

    #[test]
    fn should_optimize() {
        let res = optimize_ranges(&vec![(2..=9), (0..=4), (5..=11), (100..=122)]);
        assert_eq!(vec![(0..=11), (100..=122)], res)
    }

    #[test]
    fn should_iterate_lazily() {
        let ranges = vec![(33..=45), (77..=82)];
        let char_set = CharacterSet::new(&ranges);
        let result: Vec<char> = char_set.collect();
        assert_eq!(
            vec![
                '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', 'M', 'N', 'O',
                'P', 'Q', 'R'
            ],
            result
        );
    }
}
