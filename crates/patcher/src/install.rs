//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Installs the Portal 2 build of Hammer++ into INFRA together with the interface adapter,
//			or removes it by the manifest of the installed files
//
//=============================================================================//

use crate::pe;
use std::cmp::Ordering;
use std::path::{ Path, PathBuf };

const DLL_PATH:     &[u8] = b"hammerplusplus/bin/hammerplusplus_dll.dll";
const ADAPTER_PATH: &[u8] = b"hammerplusplus/bin/infra_hpp_adapter.dll";
const ADAPTER_FILE: &str = "infra_hpp_adapter.dll";
const ADAPTER_LOG:  &str = "hammerplusplus/bin/infra_hpp_adapter.log";
const ADAPTER:      &[u8] = include_bytes!( env!( "INFRA_HPP_ADAPTER" ) );
const MANIFEST:     &str = "hammerplusplus/infra_hpp_manifest.txt";
const VERSION_FILE: &str = "hammerplusplus/infra_hpp_version.txt";

// Shipped by earlier versions of the patcher: steam_api.dll before the adapter redirected SteamClient(),
// the adapter under its former name and a .cmd launcher, replaced by the Launch button
const LEGACY_FILES: [&str; 4] =
[
	"hammerplusplus/bin/steam_api.dll",
	"hammerplusplus/bin/infra_hpp_shim.dll",
	"hammerplusplus/bin/infra_hpp_shim.log",
	"hammerplusplus_infra.cmd",
];

pub const VERSION: &str = env!( "CARGO_PKG_VERSION" );

//-----------------------------------------------------------------------------
// Folders
//-----------------------------------------------------------------------------

pub fn is_game( dir: &Path ) -> bool
{
	dir.join( "bin" ).join( "hammer.exe" ).is_file()
}

// The INFRA folder for a typed path: the folder itself or the one above its bin
pub fn game_dir( input: &Path ) -> Option<PathBuf>
{
	if !input.is_absolute()
	{
		return None;
	}

	let found = [ Some( input ), input.parent() ].into_iter()
		.flatten()
		.find( |dir| is_game( dir ) )
		.map( Path::to_path_buf );

	found
}

// The bin folder of a Hammer++ build: the folder itself or its bin
pub fn hpp_bin( input: &Path ) -> Option<PathBuf>
{
	if !input.is_absolute()
	{
		return None;
	}

	let found = [ input.to_path_buf(), input.join( "bin" ) ].into_iter()
		.find( |dir| dir.join( "hammerplusplus.exe" ).is_file() );

	found
}

fn is_hpp_build( entry: &std::fs::DirEntry ) -> bool
{
	let name = entry.file_name().to_string_lossy().to_lowercase();

	name.starts_with( "hammerplusplus_portal2_build" ) && hpp_bin( &entry.path() ).is_some()
}

// The newest hammerplusplus_portal2_buildNNNN next to the installer or in the current folder
pub fn find_hpp_build() -> Option<PathBuf>
{
	let exe_dir = std::env::current_exe().ok().and_then( |exe| exe.parent().map( Path::to_path_buf ) );
	let roots   = [ exe_dir, std::env::current_dir().ok() ];

	let mut found = Vec::new();

	for root in roots.into_iter().flatten()
	{
		let Ok( entries ) = std::fs::read_dir( &root ) else
		{
			continue;
		};

		for entry in entries.flatten()
		{
			if is_hpp_build( &entry )
			{
				found.push( entry.path() );
			}
		}
	}

	found.sort();

	found.pop()
}

//-----------------------------------------------------------------------------
// Versions
//-----------------------------------------------------------------------------

pub fn is_installed( game: &Path ) -> bool
{
	game.join( "bin" ).join( MANIFEST ).is_file()
}

// None for installations made before the version file appeared
pub fn installed_version( game: &Path ) -> Option<String>
{
	let text    = std::fs::read_to_string( game.join( "bin" ).join( VERSION_FILE ) ).ok()?;
	let version = text.trim().to_string();

	Some( version )
}

fn parse_version( version: &str ) -> Vec<u32>
{
	version.split( '.' ).map( |part| part.parse().unwrap_or( 0 ) ).collect()
}

pub fn compare_versions( left: &str, right: &str ) -> Ordering
{
	parse_version( left ).cmp( &parse_version( right ) )
}

//-----------------------------------------------------------------------------
// Validation
//-----------------------------------------------------------------------------

fn read_file( path: &Path ) -> Result<Vec<u8>, String>
{
	std::fs::read( path ).map_err( |error| format!( "{}: {error}", path.display() ) )
}

fn write_file( path: &Path, data: impl AsRef<[u8]> ) -> Result<(), String>
{
	std::fs::write( path, data ).map_err( |error| format!( "{}: {error}", path.display() ) )
}

fn validate_launcher( bin: &Path ) -> Result<(), String>
{
	let exe = read_file( &bin.join( "hammerplusplus.exe" ) )?;

	if pe::find_all( &exe, DLL_PATH ).len() != 1
	{
		return Err( "hammerplusplus.exe has no path to hammerplusplus_dll.dll: unknown launcher version".into() );
	}

	Ok( () )
}

fn validate_dll( bin: &Path ) -> Result<(), String>
{
	let dll = read_file( &bin.join( "hammerplusplus" ).join( "bin" ).join( "hammerplusplus_dll.dll" ) )?;

	if !pe::contains( &dll, b"Release_PORTAL2" ) || !pe::contains( &dll, b"Hammer002" )
	{
		return Err( "Not the Portal 2 build of Hammer++: the adapter supports only that one".into() );
	}

	Ok( () )
}

pub fn validate_build( bin: &Path ) -> Result<(), String>
{
	validate_launcher( bin )?;
	validate_dll( bin )
}

fn validate_adapter() -> Result<(), String>
{
	if !pe::Pe::parse( ADAPTER ).is_some_and( |pe| pe.is_x86() )
	{
		return Err( format!( "The embedded {ADAPTER_FILE} is not a 32-bit DLL" ) );
	}

	Ok( () )
}

//-----------------------------------------------------------------------------
// Install
//-----------------------------------------------------------------------------

// Path relative to the INFRA bin folder, with forward slashes as in the manifest
fn manifest_path( target: &Path, root: &Path ) -> String
{
	target.strip_prefix( root ).unwrap_or( target ).to_string_lossy().replace( '\\', "/" )
}

// Keeps user-edited settings such as keybindings on reinstall
fn is_kept_config( relative: &str, target: &Path ) -> bool
{
	relative.ends_with( ".cfg" ) && target.exists()
}

fn copy_tree( from: &Path, to: &Path, root: &Path, installed: &mut Vec<String> ) -> Result<(), String>
{
	std::fs::create_dir_all( to ).map_err( |error| format!( "{}: {error}", to.display() ) )?;

	let entries = std::fs::read_dir( from ).map_err( |error| format!( "{}: {error}", from.display() ) )?;

	for entry in entries.flatten()
	{
		let source = entry.path();
		let target = to.join( entry.file_name() );

		if source.is_dir()
		{
			copy_tree( &source, &target, root, installed )?;

			continue;
		}

		let relative = manifest_path( &target, root );

		if is_kept_config( &relative, &target )
		{
			continue;
		}

		std::fs::copy( &source, &target ).map_err( |error| format!( "{}: {error}", target.display() ) )?;

		installed.push( relative );
	}

	Ok( () )
}

// The adapter path with its terminator fits in the DLL path, the rest is zero filled
fn patch_launcher( exe: &Path ) -> Result<(), String>
{
	let mut data = read_file( exe )?;

	let positions = pe::find_all( &data, DLL_PATH );

	let [ position ] = positions[ .. ] else
	{
		return Err( "The path to hammerplusplus_dll.dll is not found in the launcher".into() );
	};

	let mut replacement = ADAPTER_PATH.to_vec();
	replacement.resize( DLL_PATH.len(), 0 );

	data[ position..position + DLL_PATH.len() ].copy_from_slice( &replacement );

	write_file( exe, data )
}

// Only the files the previous installation recorded
fn remove_legacy_files( bin: &Path )
{
	let manifest = std::fs::read_to_string( bin.join( MANIFEST ) ).unwrap_or_default();

	for legacy in LEGACY_FILES
	{
		if manifest.lines().any( |line| line == legacy )
		{
			let _ = std::fs::remove_file( bin.join( legacy ) );
		}
	}
}

fn write_manifest( bin: &Path, mut installed: Vec<String> ) -> Result<usize, String>
{
	installed.extend( [ format!( "hammerplusplus/bin/{ADAPTER_FILE}" ), ADAPTER_LOG.to_string(), VERSION_FILE.to_string() ] );
	installed.sort();
	installed.dedup();

	write_file( &bin.join( MANIFEST ), installed.join( "\n" ) + "\n" )?;

	Ok( installed.len() )
}

// Installs over a previous installation as well
pub fn install( game: &Path, source: &Path ) -> Result<String, String>
{
	validate_build( source )?;
	validate_adapter()?;

	let bin = game.join( "bin" );

	remove_legacy_files( &bin );

	let mut installed = Vec::new();

	copy_tree( source, &bin, &bin, &mut installed )?;
	patch_launcher( &bin.join( "hammerplusplus.exe" ) )?;

	write_file( &bin.join( "hammerplusplus" ).join( "bin" ).join( ADAPTER_FILE ), ADAPTER )?;
	write_file( &bin.join( VERSION_FILE ), format!( "{VERSION}\n" ) )?;

	let count = write_manifest( &bin, installed )?;

	let message = format!( "Installed {count} files. Start Hammer++ with Launch" );

	Ok( message )
}

//-----------------------------------------------------------------------------
// Uninstall and launch
//-----------------------------------------------------------------------------

pub fn uninstall( game: &Path ) -> Result<String, String>
{
	let bin      = game.join( "bin" );
	let manifest = bin.join( MANIFEST );

	let list = std::fs::read_to_string( &manifest ).map_err( |_| "No installation found: the manifest is missing".to_string() )?;

	let mut removed = 0;

	for relative in list.lines().filter( |line| !line.is_empty() )
	{
		if std::fs::remove_file( bin.join( relative ) ).is_ok()
		{
			removed += 1;
		}
	}

	std::fs::remove_file( &manifest ).map_err( |error| format!( "{}: {error}", manifest.display() ) )?;

	let message = format!( "Removed {removed} files. Hammer++ settings in bin\\hammerplusplus are kept" );

	Ok( message )
}

pub fn launcher_exe( game: &Path ) -> PathBuf
{
	game.join( "bin" ).join( "hammerplusplus.exe" )
}

// The game folder, so Hammer++ opens INFRA whatever its own settings say
pub fn launch_args( game: &Path ) -> String
{
	format!( "-game \"{}\"", game.join( "infra" ).display() )
}
