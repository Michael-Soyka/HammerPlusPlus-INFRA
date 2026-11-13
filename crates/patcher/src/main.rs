//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Installer window for the Portal 2 build of Hammer++ in INFRA
//
//=============================================================================//

#![ windows_subsystem = "windows" ]

mod authors;
mod avatar;
mod background;
mod gdi;
mod gdiplus;
mod install;
mod palette;
mod pe;
mod steam;
mod ui;

fn main()
{
	ui::run();
}
