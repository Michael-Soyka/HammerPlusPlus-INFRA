//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Command line installer of the Portal 2 build of Hammer++ in INFRA, without the window
//
//=============================================================================//

use infra_hpp_patcher::cli;
use std::process::ExitCode;

fn main() -> ExitCode
{
	cli::run()
}
