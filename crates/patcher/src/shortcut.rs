//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Desktop shortcut to the installed Hammer++: finds an existing one or creates it
//
//=============================================================================//

use crate::install;
use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::path::{ Path, PathBuf };
use windows::Win32::System::Com::{ CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree, IPersistFile, STGM_READ };
use windows::Win32::UI::Shell::{ FOLDERID_Desktop, FOLDERID_PublicDesktop, IShellLinkW, KF_FLAG_DEFAULT, SHGetKnownFolderPath, ShellLink };
use windows::core::{ GUID, HSTRING, Interface };

const FILE_STEM:   &str = "Hammer++ INFRA";
const DESCRIPTION: &str = "Hammer++ for INFRA";

// "Hammer++ INFRA (2).lnk" and on when the name is taken
const MAX_FILE_NUMBER: u32 = 99;

// Long enough for any target a shortcut stores
const TARGET_LENGTH: usize = 32768;

// INFOTIPSIZE: the longest description
const DESCRIPTION_LENGTH: usize = 1024;

// Read from a .lnk file
struct Link
{
	target:      String,
	description: String,
}

fn known_folder( id: &GUID ) -> Option<PathBuf>
{
	let path = unsafe { SHGetKnownFolderPath( id, KF_FLAG_DEFAULT, None ) }.ok()?;
	let text = unsafe { path.to_string() };

	unsafe { CoTaskMemFree( Some( path.0.cast_const().cast() ) ) };

	let folder = text.ok().map( PathBuf::from );

	folder
}

fn new_link() -> windows::core::Result<IShellLinkW>
{
	unsafe { CoCreateInstance( &ShellLink, None, CLSCTX_INPROC_SERVER ) }
}

// The launcher as a full path: no forward slashes or dots left from a typed game folder
fn launcher( game: &Path ) -> Option<PathBuf>
{
	std::path::absolute( install::launcher_exe( game ) ).ok()
}

fn same_path( left: &str, right: &Path ) -> bool
{
	std::path::absolute( left ).is_ok_and( |left| left.to_string_lossy().to_lowercase() == right.to_string_lossy().to_lowercase() )
}

// Regular files only: no folders or symbolic links named *.lnk
fn link_files( folder: &Path ) -> Vec<PathBuf>
{
	let Ok( entries ) = std::fs::read_dir( folder ) else
	{
		return Vec::new();
	};

	let files = entries.flatten()
		.filter( |entry| entry.file_type().is_ok_and( |kind| kind.is_file() ) )
		.map( |entry| entry.path() )
		.filter( |path| path.extension().is_some_and( |extension| extension.eq_ignore_ascii_case( "lnk" ) ) )
		.collect();

	files
}

fn from_wide( buffer: &[u16] ) -> String
{
	let length = buffer.iter().position( |&c| c == 0 ).unwrap_or( buffer.len() );

	String::from_utf16_lossy( &buffer[ ..length ] )
}

// The stored target as is: a missing one is not searched for
fn read_link( link: &IShellLinkW, file: &Path ) -> Option<Link>
{
	let persist: IPersistFile = link.cast().ok()?;

	unsafe { persist.Load( &HSTRING::from( file ), STGM_READ ) }.ok()?;

	let mut target      = vec![ 0u16; TARGET_LENGTH ];
	let mut description = vec![ 0u16; DESCRIPTION_LENGTH ];

	unsafe { link.GetPath( &mut target, std::ptr::null_mut(), 0 ) }.ok()?;
	unsafe { link.GetDescription( &mut description ) }.ok()?;

	let found = Link { target: from_wide( &target ), description: from_wide( &description ) };

	Some( found )
}

fn find( game: &Path, folders: &[GUID], is_wanted: impl Fn(&Link) -> bool ) -> Vec<PathBuf>
{
	let ( Ok( link ), Some( launcher ) ) = ( new_link(), launcher( game ) ) else
	{
		return Vec::new();
	};

	let found = folders.iter()
		.filter_map( known_folder )
		.flat_map( |folder| link_files( &folder ) )
		.filter( |file| read_link( &link, file ).is_some_and( |read| same_path( &read.target, &launcher ) && is_wanted( &read ) ) )
		.collect();

	found
}

// Any shortcut to the launcher of this INFRA on the user or the public desktop, the user's own included
pub fn exists( game: &Path ) -> bool
{
	!find( game, &[ FOLDERID_Desktop, FOLDERID_PublicDesktop ], |_| true ).is_empty()
}

// Only the ones create() made: on the user desktop, with its description. Found before the launcher is removed
pub fn find_own( game: &Path ) -> Vec<PathBuf>
{
	find( game, &[ FOLDERID_Desktop ], |read| read.description == DESCRIPTION )
}

// Nothing while the launcher is still there: the uninstall did not remove it
pub fn remove( game: &Path, files: &[PathBuf] ) -> usize
{
	if install::launcher_exe( game ).exists()
	{
		return 0;
	}

	let removed = files.iter().filter( |file| std::fs::remove_file( file ).is_ok() ).count();

	removed
}

// Starts Hammer++ the way the Launch button does
fn save( game: &Path, launcher: &Path, file: &Path ) -> windows::core::Result<()>
{
	let link     = new_link()?;
	let launcher = HSTRING::from( launcher );
	let bin      = std::path::absolute( game.join( "bin" ) )?;

	unsafe
	{
		link.SetPath( &launcher )?;
		link.SetArguments( &HSTRING::from( install::launch_args( game ) ) )?;
		link.SetWorkingDirectory( &HSTRING::from( bin.as_path() ) )?;
		link.SetIconLocation( &launcher, 0 )?;
		link.SetDescription( &HSTRING::from( DESCRIPTION ) )?;
	}

	let persist: IPersistFile = link.cast()?;

	unsafe { persist.Save( &HSTRING::from( file ), true ) }
}

fn file_name( number: u32 ) -> String
{
	if number == 1
	{
		return format!( "{FILE_STEM}.lnk" );
	}

	let name = format!( "{FILE_STEM} ({number}).lnk" );

	name
}

// A new empty file under a free name: an existing file is never written over
fn reserve_file( desktop: &Path ) -> Result<PathBuf, String>
{
	for number in 1..=MAX_FILE_NUMBER
	{
		let file = desktop.join( file_name( number ) );

		match OpenOptions::new().write( true ).create_new( true ).open( &file )
		{
			Ok( _ ) => return Ok( file ),
			Err( error ) if error.kind() == ErrorKind::AlreadyExists => continue,
			Err( error ) => return Err( format!( "{}: {error}", file.display() ) ),
		}
	}

	Err( format!( "No free name for {FILE_STEM}.lnk on the desktop" ) )
}

pub fn create( game: &Path ) -> Result<PathBuf, String>
{
	let launcher = launcher( game ).filter( |launcher| launcher.is_file() ).ok_or( "hammerplusplus.exe is not installed" )?;
	let desktop  = known_folder( &FOLDERID_Desktop ).ok_or( "The desktop folder is not found" )?;
	let file     = reserve_file( &desktop )?;

	if let Err( error ) = save( game, &launcher, &file )
	{
		// Only the empty file reserved above
		let _ = std::fs::remove_file( &file );

		return Err( format!( "{}: {error}", file.display() ) );
	}

	Ok( file )
}
