const INVERT: &str = "\x1b[0;7m";
const NORMAL: &str = "\x1b[27m";

lazy_static::lazy_static! {
	pub static ref ANSI_REGEX: regex::Regex = regex::Regex::new(
		"[\\u001b\\u009b]\\[[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-ORZcf-nqry=><]"
	).unwrap();
}

fn main() {
	// So first - get a strip that's stripped of ansi escapes
	// if it doesn't contain the pattern, return
	// Iterate through orig string, get Vec<(usize, &str)> where .0 == index of escape in stripped
	//     string, and .1 == escape string itself
	// Get indexes of start and end of each match
	// For each match:
	//     Insert ansi invert things into stripped string
	// 
	// For each escape:
	//     Find where it would be placed in stripped string, based on how many matches and escapes appear
	//         before it, taking into account how long an ansi invert escape is
	//     If it's at the same place as the start of an invert, do nothing
	//     If it's within an invert, do nothing
	//     If it's at the end of an invert, place it after the end invert and add it to a vec to
	//         keep track
	// Return the stripped string

	let mut args = std::env::args();
	// Drop the first one, since it's the exec name
	let _ = args.next().unwrap();
	let orig_str = args.next().unwrap();
	let search_term = args.next().unwrap();

	highlight_ansi_insensitive(orig_str, &search_term);
}

pub fn highlight_ansi_insensitive<T: Into<String>>(orig_str: T, search_term: &str) -> String {
	let orig_str = orig_str.into();
	let search_regex = regex::Regex::new(search_term).unwrap();

	// Remove all ansi escapes so we can look through it as if it had none
	let stripped_str = ANSI_REGEX.replace_all(&orig_str, "");

	// if it doesn't match, don't even try. Just return.
	if !search_regex.is_match(&stripped_str) {
		return orig_str
	}

	// sum_width is used to calculate the total width of the ansi escapes
	// up to the point in the original string where it is being used
	let mut sum_width = 0;

	// find all ansi escapes in the original string, and map them
	// to a Vec<(usize, &str)> where
	//   .0 == the start index in the STRIPPED string
	//   .1 == the escape sequence itself
	let escapes = ANSI_REGEX.find_iter(&orig_str).map(|escape| {
		let start = escape.start();
		let as_str = escape.as_str();
		let ret = (start - sum_width, as_str);
		sum_width += as_str.len();
		ret
	}).collect::<Vec<_>>();

	// The matches of the term you're looking for, so that you can easily determine where
	// the invert attributes will be placed
	let matches = search_regex.find_iter(&stripped_str)
		.map(|c| [c.start(), c.end()])
		.flatten()
		.collect::<Vec<_>>();

	// Highlight all the instances of the search term in the stripped string
	// by inverting their background/foreground colors
	let mut inverted = search_regex.replace_all(&stripped_str, |caps: &regex::Captures| {
		format!("{}{}{}", INVERT, &caps[0], NORMAL)
	}).to_string();

	// inserted_escs_len == the total length of the ascii escapes which have been re-inserted
	// into the stripped string at the point where it is being checked.
	let mut inserted_escs_len = 0;
	for esc in escapes {
		// Find how many invert|normal markers appear before this escape
		let match_count = matches.iter().take_while(|m| **m <= esc.0).count();

		if match_count % 2 == 1 {
			// if == 1, then it's either at the same spot as the start of an invert, or in the
			// middle of an invert. Either way we don't want to place it in.
			continue;
		}

		// find the number of invert strings and number of uninvert strings that have been
		// inserted up to this point in the string
		let num_invert = match_count / 2;
		let num_normal = match_count - num_invert;

		// calculate the index which this escape should be re-inserted at by adding
		// its position in the stripped string to the total length of the ansi escapes
		// (both highlighting and the ones from the original string).
		let pos = esc.0 + inserted_escs_len + (num_invert * INVERT.len()) + (num_normal * NORMAL.len());

		// insert the escape back in
		inverted.insert_str(pos, esc.1);

		// increment the length of the escapes inserted back in
		inserted_escs_len += esc.1.len();
	}

	println!("orig: {orig_str}\nfixed: {inverted}");
	inverted
}

#[cfg(test)]
mod tests {
	pub use super::*;

	// generic escape code
	const ESC: &str = "\x1b[34m";
	const NONE: &str = "\x1b[0m";

	#[test]
	pub fn no_match() {
		let orig = "no match";
		let res = highlight_ansi_insensitive(orig, "test");
		assert_eq!(res, orig.to_string());
	}

	#[test]
	pub fn single_match_no_esc() {
		let res = highlight_ansi_insensitive("this is a test", " a ");
		assert_eq!(res, format!("this is{} a {}test", INVERT, NORMAL));
	}

	#[test]
	pub fn multi_match_no_esc() {
		let res = highlight_ansi_insensitive("test another test", "test");
		assert_eq!(res, format!("{i}test{n} another {i}test{n}", i = INVERT, n = NORMAL));
	}

	#[test]
	pub fn esc_outside_match() {
		let res = highlight_ansi_insensitive(format!("{}color{} and test", ESC, NONE), "test");
		assert_eq!(res, format!("{}color{} and {}test{}", ESC, NONE, INVERT, NORMAL));
	}

	#[test]
	pub fn esc_end_in_match() {
		let orig = format!("this {}is a te{}st", ESC, NONE);
		let res = highlight_ansi_insensitive(orig, "test");
		assert_eq!(res, format!("this {}is a {}test{}", ESC, INVERT, NORMAL));
	}

	#[test]
	pub fn esc_start_in_match() {
		let orig = format!("this is a te{}st again{}", ESC, NONE);
		let res = highlight_ansi_insensitive(orig, "test");
		assert_eq!(res, format!("this is a {}test{} again{}", INVERT, NORMAL, NONE));
	}

	#[test]
	pub fn esc_around_match() {
		let orig = format!("this is {}a test again{}", ESC, NONE);
		let res = highlight_ansi_insensitive(orig, "test");
		assert_eq!(res, format!("this is {}a {}test{} again{}", ESC, INVERT, NORMAL, NONE));
	}

	#[test]
	pub fn esc_within_match() {
		let orig = format!("this is a t{}es{}t again", ESC, NONE);
		let res = highlight_ansi_insensitive(orig, "test");
		assert_eq!(res, format!("this is a {}test{} again", INVERT, NORMAL));
	}

	#[test]
	pub fn multi_escape_match() {
		let orig = format!("this {e}is a te{n}st again {e}yeah{n} test", e = ESC, n = NONE);
		let res = highlight_ansi_insensitive(orig, "test");
		assert_eq!(res, format!("this {e}is a {i}test{n} again {e}yeah{nn} {i}test{n}", e = ESC, i = INVERT, n = NORMAL, nn = NONE));
	}
}
