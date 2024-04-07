/// This module contains helper functions for string manipulation.
use bytes::BytesMut;
use tokio::runtime::Handle;

/// Constants for ASCII values of lowercase and uppercase letters
const LOWER_BEGIN: u8 = b'a';
const LOWER_END: u8 = b'z';
const UPPER_TO_LOWER: u8 = b'a' - b'A';

/// Converts all lowercase ASCII characters in the input to uppercase.
///
/// # Arguments
///
/// * `input` - A mutable byte slice that will be converted to uppercase.
pub(crate) fn upper(input: &mut [u8]) {
    for b in input {
        if *b < LOWER_BEGIN || *b > LOWER_END {
            continue;
        }
        *b -= UPPER_TO_LOWER;
    }
}

/// Converts a usize to a string and appends it to a BytesMut buffer.
///
/// # Arguments
///
/// * `input` - The usize to convert to a string.
/// * `buf` - The buffer to append the string to.
pub(crate) fn itoa(input: usize, buf: &mut BytesMut) {
    let value = format!("{}", input);
    buf.extend_from_slice(value.as_bytes());
}

/// Trims a hash tag from a key.
///
/// # Arguments
///
/// * `key` - The key to trim the hash tag from.
/// * `hash_tag` - The hash tag to trim.
///
/// # Returns
///
/// * A byte slice representing the key with the hash tag trimmed.
#[inline]
pub(crate) fn trim_hash_tag<'a>(key: &'a [u8], hash_tag: &[u8]) -> &'a [u8] {
    if hash_tag.len() != 2 {
        return key;
    }
    if let Some(begin) = key.iter().position(|x| *x == hash_tag[0]) {
        if let Some(end_offset) = key[begin..].iter().position(|x| *x == hash_tag[1]) {
            // to avoid abc{}de
            if end_offset > 1 {
                return &key[begin + 1..begin + end_offset];
            }
        }
    }
    key
}

// get_runtime_handle returns the current runtime handle from tokio.
// It panics if the runtime handle is not available so it should only be used in contexts where
// the runtime handle is guaranteed to be available.
pub fn get_runtime_handle() -> Handle {
    Handle::try_current().expect("runtime handle must be available here")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upper_empty() {
        let mut input: [u8; 0] = [];
        upper(&mut input);
        assert_eq!(input, [] as [u8; 0]);
    }

    #[test]
    fn test_upper_no_lowercase() {
        let mut input = b"HELLO".to_vec();
        upper(&mut input);
        assert_eq!(input, b"HELLO");
    }

    #[test]
    fn test_upper_single_lowercase() {
        let mut input = b"hello".to_vec();
        upper(&mut input);
        assert_eq!(input, b"HELLO");
    }

    #[test]
    fn test_upper_multiple_lowercase() {
        let mut input = b"hello world".to_vec();
        upper(&mut input);
        assert_eq!(input, b"HELLO WORLD");
    }

    #[test]
    fn test_upper_mixed_case() {
        let mut input = b"HeLlO WoRlD".to_vec();
        upper(&mut input);
        assert_eq!(input, b"HELLO WORLD");
    }

    // Test function for myitoa function.
    #[test]
    fn test_itoa_ok() {
        let mut buf = BytesMut::with_capacity(1);

        itoa(10, &mut buf);
        assert_eq!("10", String::from_utf8_lossy(buf.as_ref()));
        buf.clear();

        itoa(std::usize::MAX, &mut buf);
        assert_eq!(
            "18446744073709551615",
            String::from_utf8_lossy(buf.as_ref())
        );
        buf.clear();
    }
}
