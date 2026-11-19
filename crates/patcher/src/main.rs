//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Installer window for the Portal 2 build of Hammer++ in INFRA,
//			the command line when started with arguments
//
//=============================================================================//

#![ windows_subsystem = "windows" ]

use infra_hpp_patcher::{ cli, ui };
use std::process::ExitCode;

fn main() -> ExitCode
{
	// Only from a console: a folder dropped on the exe still opens the window
	if std::env::args_os().len() > 1 && cli::attach_parent_console()
	{
		return cli::run_in_parent_console();
	}

	ui::run();

	ExitCode::SUCCESS
}
