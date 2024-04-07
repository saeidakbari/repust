/// A struct representing a range with a beginning and an end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub begin: u32,
    pub end: u32,
}

impl Range {
    pub fn new(begin: usize, end: usize) -> Range {
        Range {
            begin: begin as u32,
            end: end as u32,
        }
    }

    #[inline(always)]
    pub fn set_begin(&mut self, begin: usize) {
        self.begin = begin as u32;
    }

    #[inline(always)]
    pub fn set_end(&mut self, end: usize) {
        self.end = end as u32;
    }

    #[inline(always)]
    pub fn begin(self) -> usize {
        self.begin as usize
    }

    #[inline(always)]
    pub fn end(self) -> usize {
        self.end as usize
    }

    #[inline]
    pub fn range(self) -> usize {
        (self.end - self.begin) as usize
    }
}

impl Default for Range {
    fn default() -> Range {
        Range::new(0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range() {
        let range = Range::new(5, 10);

        assert_eq!(range.begin(), 5);
        assert_eq!(range.end(), 10);
        assert_eq!(range.range(), 5);
    }

    #[test]
    fn test_set_begin() {
        let mut range = Range::new(0, 0);
        range.set_begin(5);

        assert_eq!(range.begin(), 5);
    }

    #[test]
    fn test_set_end() {
        let mut range = Range::new(0, 0);
        range.set_end(10);

        assert_eq!(range.end(), 10);
    }

    #[test]
    fn test_default() {
        let range = Range::default();

        assert_eq!(range.begin(), 0);
        assert_eq!(range.end(), 0);
        assert_eq!(range.range(), 0);
    }
}
