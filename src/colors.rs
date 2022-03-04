use colored::*;

//let _0: uint8 = 0;

fn red(text: String) -> ColoredString { text.truecolor(172, 32, 16) }
fn orange(text: String) -> ColoredString { text.truecolor(192, 120, 16) }
fn yellow(text: String) -> ColoredString { text.truecolor(172, 148, 16) }
fn green(text: String) -> ColoredString { text.truecolor() }
fn blue(text: String) -> ColoredString { text.truecolor() }
fn purple(text: String) -> ColoredString { text.truecolor() }
fn grey(text: String) -> ColoredString { text.truecolor(32, 32, 32) }

/*
18  const codeRed = process.env.BUZULI_COLOR_RED || (light ? 'a10' : dark ? 'd43' : 'b21')
  1 const codeOrange = process.env.BUZULI_COLOR_ORANGE || (light ? 'b50' : dark ? 'e83' : 'c61')
  2 const codeYellow = process.env.BUZULI_COLOR_YELLOW || (light ? 'c90' : dark ? 'dc3' : 'ba1')
  3 const codeGreen = process.env.BUZULI_COLOR_GREEN || (light ? '380' : dark ? '6b3' : '491')
  4 const codeBlue = process.env.BUZULI_COLOR_BLUE || (light ? '07a' : dark ? '3ad' : '18b')
  5 const codePurple = process.env.BUZULI_COLOR_PURPLE || (light ? '62c' : dark ? '95f' : '73d')
  6 const codeGrey = process.env.BUZULI_COLOR_GREY || process.env.BUZULI_COLOR_GRAY || (light ? '777' : dark ? 'aaa' : '888')
*/