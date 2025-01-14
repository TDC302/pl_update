use std::{ffi::OsString, os::windows::ffi::OsStrExt};

use widestring::{error::Utf16Error, Utf16Str, Utf16String};

pub trait StringExts<T, Slice: ?Sized> {
    type EncodingError;
    fn first_match(&self, pat: T) -> Option<(usize, usize)>;
    fn last_match(&self, pat: T) -> Option<(usize, usize)>;
    fn contains(&self, pat: T) -> bool;
    fn eq_ignore_case(&self, pat: T) -> bool;
    fn starts_with(&self, pat: T) -> bool;
    fn ends_with(&self, pat: T) -> bool;
    fn rsplit_once(&self, pat: T) -> Option<(&Slice, &Slice)>;
    fn from_os_string(data: OsString) -> Result<T, Self::EncodingError>;

}

impl StringExts<Utf16String, Utf16Str> for Utf16String {
    type EncodingError = Utf16Error;

    fn first_match(&self, pat: Utf16String) -> Option<(usize, usize)> {
        let self_bytes = self.as_slice();
        let pat_bytes = pat.as_slice();
        let mut char_idx = 0;
        for i in 0..self_bytes.len() {
            if self_bytes[i] == pat_bytes[char_idx] {
                char_idx += 1;
                if char_idx >= pat_bytes.len() {
                    return Some((i+1-pat_bytes.len(), i+1));
                }
            } else {
                char_idx = 0;
            }
        }
        None
    }

    fn last_match(&self, pat: Utf16String) -> Option<(usize, usize)> {
        let self_bytes = self.as_slice();
        let pat_bytes = pat.as_slice();
        let mut char_idx = pat_bytes.len()-1;

        let rng = 0..self_bytes.len();
        for i in rng.rev() {
            if self_bytes[i] == pat_bytes[char_idx] {
                if char_idx <= 0 {
                    return Some((i, i+pat_bytes.len()));
                }
                char_idx -= 1;
            } else {
                char_idx = pat_bytes.len()-1;
            }
        }
        None
    }

    fn contains(&self, pat: Utf16String) -> bool {
        self.first_match(pat).is_some()
    }
    
    fn eq_ignore_case(&self, pat: Utf16String) -> bool {
        self.to_lowercase() == pat.to_lowercase()
    }
    
    fn starts_with(&self, pat: Utf16String) -> bool {
        self.as_slice().starts_with(pat.as_slice())
    }

    fn ends_with(&self, pat: Utf16String) -> bool {
        self.as_slice().ends_with(pat.as_slice())
    }
    
    fn from_os_string(data: OsString) -> Result<Utf16String, Utf16Error> {
        let buf: Vec<_> = data.encode_wide().collect();
        Utf16String::from_vec(buf)
    }
    
    fn rsplit_once(&self, pat: Utf16String) -> Option<(&Utf16Str, &Utf16Str)> {
        if let Some((start_idx, _)) =  self.last_match(pat) {
            Some(self.split_at(start_idx))
        } else {
            None
        }
    }
}


