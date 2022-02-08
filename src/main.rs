fn main() {
	// So first - get a strip that's stripped of ansi escapes
	// if it doesn't contain the pattern, return
	// Iterate through orig string, get Vec<(usize, usize)> where .0 == index of escape in stripped
	//     string, and .1 == length of escape
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

	const INVERT: &str = "\x1b[0;7m";
	const NORMAL: &str = "\x1b[27m";

	let ansi_regex = regex::Regex::new(
		"[\\u001b\\u009b]\\[[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-ORZcf-nqry=><]"
	).unwrap();

	let mut args = std::env::args();
	// Drop the first one, since it's the exec name
	let _ = args.next().unwrap();
	let orig_str = args.next().unwrap();
	let search_term = args.next().unwrap();
	let search_regex = regex::Regex::new(&search_term).unwrap();

	let stripped_str = ansi_regex.replace_all(&orig_str, "");

	if !search_regex.is_match(&stripped_str) {
		println!("{}", orig_str);
		return
	}

	let mut sum_width = 0;
	let escapes = ansi_regex.find_iter(&orig_str).map(|escape| {
		let start = escape.start();
		let len = escape.end() - start;
		let ret = (start - sum_width, len, escape.as_str());
		sum_width += len;
		ret
	}).collect::<Vec<_>>();

	// The matches of the term you're looking for, so that you can easily determine where
	// the invert attributes will be placed
	let matches = search_regex.find_iter(&stripped_str)
		.map(|c| [c.start(), c.end()])
		.flatten()
		.collect::<Vec<_>>();

	let mut inverted = search_regex.replace_all(&stripped_str, |caps: &regex::Captures| {
		format!("{}{}{}", INVERT, &caps[0], NORMAL)
	}).to_string();

	let mut inserted_escs_len = 0;
	for esc in escapes {
		// Find how many invert|normal markers appear before this escape
		let match_count = matches.iter().take_while(|m| **m <= esc.0).count();

		if match_count % 2 == 1 {
			// if == 1, then it's either at the same spot as the start of an invert, or in the
			// middle of an invert. Either way we don't want to place it in.
			continue;
		}

		let num_invert = match_count / 2;
		let num_normal = match_count - num_invert;

		let pos = esc.0 + inserted_escs_len + (num_invert * INVERT.len()) + (num_normal * NORMAL.len());

		inverted.insert_str(pos, esc.2);

		inserted_escs_len += esc.1;
	}

	println!("orig: {}\nfixed: {}", orig_str, inverted);
}
