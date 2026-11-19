//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Command line: install, remove, launch and status, done the way the window does them
//
//=============================================================================//

use crate::{ install, process, shortcut, steam };
use std::ffi::OsString;
use std::io::{ BufRead, Write };
use std::os::windows::process::CommandExt;
use std::path::{ Path, PathBuf };
use std::process::ExitCode;
use std::ptr::{ null, null_mut };
use std::time::{ Duration, Instant };
use windows_sys::Win32::Foundation::{ CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE };
use windows_sys::Win32::Storage::FileSystem::{ CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING };
use windows_sys::Win32::System::Com::{ COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize };
use windows_sys::Win32::System::Console::{ ATTACH_PARENT_PROCESS, AttachConsole, CONSOLE_CHARACTER_ATTRIBUTES, CONSOLE_SCREEN_BUFFER_INFO, FOREGROUND_BLUE, FOREGROUND_GREEN, FOREGROUND_INTENSITY, FOREGROUND_RED, GetConsoleProcessList, GetConsoleScreenBufferInfo, GetStdHandle, INPUT_RECORD, INPUT_RECORD_0, KEY_EVENT, KEY_EVENT_RECORD, KEY_EVENT_RECORD_0, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE, SetConsoleTextAttribute, WriteConsoleInputW };
use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_RETURN;

// Wrong arguments, as opposed to a command that failed
const USAGE_ERROR: u8 = 2;

const DEFAULT_EXE: &str = "infra-hpp-patcher-cli.exe";

const ENTER_SCAN_CODE: u16 = 0x1c;

#[ derive( Clone, Copy, PartialEq ) ]
enum Command
{
	Install,
	Remove,
	Launch,
	Status,
	Help,
	Version,
}

#[ derive( Default ) ]
struct Options
{
	game:     Option<PathBuf>,
	hpp:      Option<PathBuf>,
	shortcut: bool,
	close:    bool,
}

//-----------------------------------------------------------------------------
// Colors: console attributes, not escape sequences, which the console of Windows 8 does not read
//-----------------------------------------------------------------------------

#[ derive( Clone, Copy ) ]
enum Color
{
	Red,
	Green,
	Yellow,
	Cyan,
	Gray,
}

#[ derive( Clone, Copy ) ]
enum Stream
{
	Out,
	Err,
}

// The background bits of an attribute stay as the console has them
const FOREGROUND_MASK: CONSOLE_CHARACTER_ATTRIBUTES = 0x0f;

fn attribute( color: Color ) -> CONSOLE_CHARACTER_ATTRIBUTES
{
	match color
	{
		Color::Red => FOREGROUND_RED | FOREGROUND_INTENSITY,
		Color::Green => FOREGROUND_GREEN | FOREGROUND_INTENSITY,
		Color::Yellow => FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_INTENSITY,
		Color::Cyan => FOREGROUND_GREEN | FOREGROUND_BLUE | FOREGROUND_INTENSITY,
		Color::Gray => FOREGROUND_INTENSITY,
	}
}

// None when redirected to a file or a pipe, or when NO_COLOR asks for plain text
fn console( stream: Stream ) -> Option<(HANDLE, CONSOLE_CHARACTER_ATTRIBUTES)>
{
	if std::env::var_os( "NO_COLOR" ).is_some_and( |value| !value.is_empty() )
	{
		return None;
	}

	screen( stream )
}

// The console screen the stream writes to, with its current attribute; None when redirected
fn screen( stream: Stream ) -> Option<(HANDLE, CONSOLE_CHARACTER_ATTRIBUTES)>
{
	let id = match stream
	{
		Stream::Out => STD_OUTPUT_HANDLE,
		Stream::Err => STD_ERROR_HANDLE,
	};

	let handle = unsafe { GetStdHandle( id ) };

	let mut info: CONSOLE_SCREEN_BUFFER_INFO = unsafe { std::mem::zeroed() };

	if unsafe { GetConsoleScreenBufferInfo( handle, &mut info ) } == 0
	{
		return None;
	}

	Some( ( handle, info.wAttributes ) )
}

fn write_to( stream: Stream, text: &str )
{
	let _ = match stream
	{
		Stream::Out => std::io::stdout().write_all( text.as_bytes() ).and_then( |_| std::io::stdout().flush() ),
		Stream::Err => std::io::stderr().write_all( text.as_bytes() ),
	};
}

// Flushed before the attribute changes back, so the text gets the color
fn paint( stream: Stream, color: Color, text: &str )
{
	let Some( ( handle, original ) ) = console( stream ) else
	{
		write_to( stream, text );

		return;
	};

	unsafe { SetConsoleTextAttribute( handle, ( original & !FOREGROUND_MASK ) | attribute( color ) ) };

	write_to( stream, text );

	unsafe { SetConsoleTextAttribute( handle, original ) };
}

fn line( color: Color, text: &str )
{
	paint( Stream::Out, color, &format!( "{text}\n" ) );
}

// A label column, then the value in its color
fn field( label: &str, value: &str, color: Option<Color> )
{
	paint( Stream::Out, Color::Cyan, label );

	match color
	{
		Some( color ) => line( color, value ),
		None => println!( "{value}" ),
	}
}

fn print_error( text: &str )
{
	paint( Stream::Err, Color::Red, "error: " );

	eprintln!( "{text}" );
}

//-----------------------------------------------------------------------------
// Arguments
//-----------------------------------------------------------------------------

fn exe_name() -> String
{
	let name = std::env::current_exe().ok()
		.and_then( |exe| exe.file_name().map( |name| name.to_string_lossy().into_owned() ) )
		.unwrap_or_else( || DEFAULT_EXE.to_string() );

	name
}

fn usage() -> String
{
	let text = format!( "\
Hammer++ patcher for INFRA {version}

Usage: {exe} <command> [options]

Commands:
  install    Install Hammer++ into INFRA, or update the installation
  remove     Remove Hammer++ from INFRA; its settings in bin\\hammerplusplus are kept
  launch     Start the installed Hammer++ with INFRA
  status     Show whether Hammer++ is installed and its version

Options:
  --game <folder>    The INFRA folder; found through Steam when omitted
  --hpp <folder>     install: the hammerplusplus_portal2_buildXXXX folder;
                     found next to the patcher when omitted
  --shortcut         install: create a desktop shortcut unless there is one
  --close            remove: ask a running Hammer++ to close first and wait for it
  -h, --help         Show this help
  -V, --version      Show the version
", version = install::VERSION, exe = exe_name() );

	text
}

fn command( name: &str ) -> Option<Command>
{
	let command = match name
	{
		"install" => Command::Install,
		"remove" => Command::Remove,
		"launch" => Command::Launch,
		"status" => Command::Status,
		"help" | "-h" | "--help" => Command::Help,
		"-V" | "--version" => Command::Version,
		_ => return None,
	};

	Some( command )
}

// An option the command does not read is an error, not silently ignored
fn allows( command: Command, option: &str ) -> bool
{
	match option
	{
		"--game" => matches!( command, Command::Install | Command::Remove | Command::Launch | Command::Status ),
		"--hpp" | "--shortcut" => command == Command::Install,
		"--close" => command == Command::Remove,
		_ => false,
	}
}

fn folder( args: &mut impl Iterator<Item = OsString>, option: &str ) -> Result<PathBuf, String>
{
	args.next().map( PathBuf::from ).ok_or_else( || format!( "{option} needs a folder" ) )
}

// No arguments at all show the help
fn parse( mut args: impl Iterator<Item = OsString> ) -> Result<(Command, Options), String>
{
	let mut options = Options::default();

	let Some( first ) = args.next() else
	{
		return Ok( ( Command::Help, options ) );
	};

	let name    = first.to_string_lossy().into_owned();
	let command = command( &name ).ok_or_else( || format!( "Unknown command: {name}" ) )?;

	while let Some( arg ) = args.next()
	{
		let option = arg.to_string_lossy().into_owned();

		if !allows( command, &option )
		{
			return Err( format!( "Unknown option of {name}: {option}" ) );
		}

		match option.as_str()
		{
			"--game" => options.game = Some( folder( &mut args, &option )? ),
			"--hpp" => options.hpp = Some( folder( &mut args, &option )? ),
			"--shortcut" => options.shortcut = true,
			"--close" => options.close = true,
			_ => {}
		}
	}

	Ok( ( command, options ) )
}

//-----------------------------------------------------------------------------
// Folders
//-----------------------------------------------------------------------------

fn absolute( path: &Path ) -> Result<PathBuf, String>
{
	std::path::absolute( path ).map_err( |error| format!( "{}: {error}", path.display() ) )
}

fn find_game( input: Option<&Path> ) -> Result<PathBuf, String>
{
	let Some( input ) = input else
	{
		return steam::find_infra().and_then( |dir| install::game_dir( &dir ) ).ok_or_else( || "INFRA is not found through Steam: pass --game <folder>".to_string() );
	};

	let game = install::game_dir( &absolute( input )? ).ok_or_else( || format!( "Not an INFRA folder, there is no bin\\hammer.exe: {}", input.display() ) )?;

	Ok( game )
}

// The bin folder of the build
fn find_hpp( input: Option<&Path> ) -> Result<PathBuf, String>
{
	let folder = match input
	{
		Some( input ) => absolute( input )?,
		None => install::find_hpp_build().ok_or( "No hammerplusplus_portal2_buildXXXX folder next to the patcher: pass --hpp <folder>" )?,
	};

	let bin = install::hpp_bin( &folder ).ok_or_else( || format!( "Not a Hammer++ build, there is no hammerplusplus.exe: {}", folder.display() ) )?;

	Ok( bin )
}

fn installed_game( options: &Options ) -> Result<PathBuf, String>
{
	let game = find_game( options.game.as_deref() )?;

	if !install::is_installed( &game )
	{
		return Err( format!( "Hammer++ is not installed in {}", game.display() ) );
	}

	Ok( game )
}

//-----------------------------------------------------------------------------
// Commands
//-----------------------------------------------------------------------------

fn add_shortcut( game: &Path ) -> Result<(), String>
{
	if shortcut::exists( game )
	{
		line( Color::Gray, "There is a desktop shortcut already" );

		return Ok( () );
	}

	let file = shortcut::create( game )?;

	line( Color::Green, &format!( "Desktop shortcut created: {}", file.display() ) );

	Ok( () )
}

// A running Hammer++ holds its files: the update would stop halfway
fn run_install( options: &Options ) -> Result<(), String>
{
	let game = find_game( options.game.as_deref() )?;
	let hpp  = find_hpp( options.hpp.as_deref() )?;

	if !install::running_hammer( &game ).is_empty()
	{
		return Err( "Hammer++ is running: save your maps, close it and run install again".into() );
	}

	field( "INFRA:    ", &game.display().to_string(), None );
	field( "Hammer++: ", &hpp.display().to_string(), None );

	line( Color::Green, &install::install( &game, &hpp )? );

	if options.shortcut
	{
		add_shortcut( &game )?;
	}

	Ok( () )
}

// As the window does: Hammer++ may ask to save first, it is never killed
fn close_hammer( game: &Path ) -> Result<(), String>
{
	line( Color::Yellow, "Waiting for Hammer++ to close. If it asks to save, answer it there" );

	process::close( &install::running_hammer( game ) );

	let start = Instant::now();

	while !install::running_hammer( game ).is_empty()
	{
		if start.elapsed() >= process::CLOSE_TIMEOUT
		{
			return Err( "Hammer++ did not close, nothing was removed. Close it and run remove again".into() );
		}

		std::thread::sleep( Duration::from_millis( process::CLOSE_POLL_MS.into() ) );
	}

	Ok( () )
}

fn run_remove( options: &Options ) -> Result<(), String>
{
	let game = installed_game( options )?;

	if !install::running_hammer( &game ).is_empty()
	{
		if !options.close
		{
			return Err( "Hammer++ is running: save your maps and close it, or pass --close".into() );
		}

		close_hammer( &game )?;
	}

	line( Color::Green, &install::uninstall_with_shortcuts( &game )? );

	Ok( () )
}

fn run_launch( options: &Options ) -> Result<(), String>
{
	let game = installed_game( options )?;

	if install::is_older( install::installed_version( &game ).as_deref() )
	{
		return Err( "The installation is older than this patcher: run install first".into() );
	}

	std::process::Command::new( install::launcher_exe( &game ) )
		.raw_arg( install::launch_args( &game ) )
		.current_dir( game.join( "bin" ) )
		.spawn()
		.map_err( |error| format!( "Failed to start hammerplusplus.exe: {error}" ) )?;

	line( Color::Green, "Hammer++ started" );

	Ok( () )
}

fn run_status( options: &Options ) -> Result<(), String>
{
	let game = find_game( options.game.as_deref() )?;

	field( "INFRA:    ", &game.display().to_string(), None );

	if !install::is_installed( &game )
	{
		field( "Hammer++: ", "not installed", Some( Color::Yellow ) );

		return Ok( () );
	}

	let version = install::installed_version( &game );

	let mut freshness = ( "up to date", Color::Green );

	if install::is_older( version.as_deref() )
	{
		freshness = ( "older than this patcher, run install to update", Color::Yellow );
	}

	let mut running = ( "no", None );

	if !install::running_hammer( &game ).is_empty()
	{
		running = ( "yes", Some( Color::Yellow ) );
	}

	let ( state, color ) = freshness;

	field( "Hammer++: ", &format!( "installed, version {} ({state})", version.as_deref().unwrap_or( "unknown" ) ), Some( color ) );
	field( "Running:  ", running.0, running.1 );

	Ok( () )
}

// Two spaces end the name column of a command or an option row
fn print_row( row: &str )
{
	let text = &row[ 2.. ];
	let end  = text.find( "  " ).unwrap_or( text.len() );

	print!( "  " );
	paint( Stream::Out, Color::Cyan, &text[ ..end ] );
	println!( "{}", &text[ end.. ] );
}

// A heading is an unindented line up to its colon
fn print_heading( heading: &str )
{
	let end = heading.find( ':' ).map_or( heading.len(), |colon| colon + 1 );

	paint( Stream::Out, Color::Yellow, &heading[ ..end ] );
	println!( "{}", &heading[ end.. ] );
}

// The title, headings and names in color; continuation lines, indented deeper, stay plain
fn print_help() -> Result<(), String>
{
	let text = usage();

	for ( index, row ) in text.lines().enumerate()
	{
		if index == 0
		{
			line( Color::Green, row );
		}
		else if row.starts_with( "   " ) || row.is_empty()
		{
			println!( "{row}" );
		}
		else if row.starts_with( "  " )
		{
			print_row( row );
		}
		else
		{
			print_heading( row );
		}
	}

	Ok( () )
}

fn print_version() -> Result<(), String>
{
	line( Color::Green, install::VERSION );

	Ok( () )
}

fn execute( args: impl Iterator<Item = OsString> ) -> ExitCode
{
	let ( command, options ) = match parse( args )
	{
		Ok( parsed ) => parsed,
		Err( error ) =>
		{
			print_error( &format!( "{error}\nSee {} --help", exe_name() ) );

			return ExitCode::from( USAGE_ERROR );
		}
	};

	let result = match command
	{
		Command::Install => run_install( &options ),
		Command::Remove => run_remove( &options ),
		Command::Launch => run_launch( &options ),
		Command::Status => run_status( &options ),
		Command::Help => print_help(),
		Command::Version => print_version(),
	};

	if let Err( error ) = result
	{
		print_error( &error );

		return ExitCode::FAILURE;
	}

	ExitCode::SUCCESS
}

//-----------------------------------------------------------------------------
// Console
//-----------------------------------------------------------------------------

// For the window executable, which has no console of its own: true when it was started from one,
// and it writes there from now on
pub fn attach_parent_console() -> bool
{
	unsafe { AttachConsole( ATTACH_PARENT_PROCESS ) != 0 }
}

// CONIN$ or CONOUT$: the console itself, whatever the standard handles are redirected to
fn open_console( name: &str ) -> Option<HANDLE>
{
	let name   = crate::ui::wide( name );
	let handle = unsafe { CreateFileW( name.as_ptr(), GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, null(), OPEN_EXISTING, 0, null_mut() ) };

	if handle == INVALID_HANDLE_VALUE
	{
		return None;
	}

	Some( handle )
}

fn cursor_column() -> Option<i16>
{
	let handle = open_console( "CONOUT$" )?;

	let mut info: CONSOLE_SCREEN_BUFFER_INFO = unsafe { std::mem::zeroed() };

	let read = unsafe { GetConsoleScreenBufferInfo( handle, &mut info ) } != 0;

	unsafe { CloseHandle( handle ) };

	read.then_some( info.dwCursorPosition.X )
}

fn key( down: bool ) -> INPUT_RECORD
{
	let event = KEY_EVENT_RECORD
	{
		bKeyDown:          down.into(),
		wRepeatCount:      1,
		wVirtualKeyCode:   VK_RETURN,
		wVirtualScanCode:  ENTER_SCAN_CODE,
		uChar:             KEY_EVENT_RECORD_0 { UnicodeChar: u16::from( b'\r' ) },
		dwControlKeyState: 0,
	};

	let record = INPUT_RECORD { EventType: KEY_EVENT as u16, Event: INPUT_RECORD_0 { KeyEvent: event } };

	record
}

// Enter in the input of the shell makes it print its prompt again below the output
fn press_enter()
{
	let Some( handle ) = open_console( "CONIN$" ) else
	{
		return;
	};

	let records = [ key( true ), key( false ) ];

	let mut written = 0;

	unsafe
	{
		WriteConsoleInputW( handle, records.as_ptr(), records.len() as u32, &mut written );
		CloseHandle( handle );
	}
}

// Started from Explorer, the console closes together with the process: it stays until Enter
fn pause_in_own_console()
{
	let mut processes = [ 0u32; 2 ];

	let count = unsafe { GetConsoleProcessList( processes.as_mut_ptr(), processes.len() as u32 ) };

	if count != 1
	{
		return;
	}

	paint( Stream::Out, Color::Gray, "\nPress Enter to close" );
	let _ = std::io::stdin().lock().read_line( &mut String::new() );
}

// Shortcuts go through COM
fn execute_with_com() -> ExitCode
{
	unsafe { CoInitializeEx( null(), ( COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE ) as u32 ) };

	let code = execute( std::env::args_os().skip( 1 ) );

	unsafe { CoUninitialize() };

	code
}

// The console executable
pub fn run() -> ExitCode
{
	let code = execute_with_com();

	pause_in_own_console();

	code
}

// The window executable after attach_parent_console(). An interactive shell does not wait for it
// and prints its prompt at once: the cursor stands after the prompt. A batch file waits,
// the cursor stays at the line start, and nothing is added there
pub fn run_in_parent_console() -> ExitCode
{
	let prompted = cursor_column().is_some_and( |column| column > 0 );

	let mut shown = false;

	if prompted && screen( Stream::Out ).is_some()
	{
		println!();

		shown = true;
	}
	else if prompted && screen( Stream::Err ).is_some()
	{
		eprintln!();

		shown = true;
	}

	let code = execute_with_com();

	if shown
	{
		press_enter();
	}

	code
}
