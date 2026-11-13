//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Locates INFRA through the registry and the Steam libraries
//
//=============================================================================//

use std::path::{ Path, PathBuf };
use windows_sys::Win32::System::Registry::{ HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, REG_ROUTINE_FLAGS, RRF_RT_REG_SZ, RRF_SUBKEY_WOW6432KEY, RegGetValueW };

const INFRA_APP_ID: &str = "251110";

fn wide( text: &str ) -> Vec<u16>
{
	text.encode_utf16().chain( std::iter::once( 0 ) ).collect()
}

//-----------------------------------------------------------------------------
// Registry
//-----------------------------------------------------------------------------

fn registry_string( root: HKEY, key: &str, value: &str, flags: REG_ROUTINE_FLAGS ) -> Option<String>
{
	let key   = wide( key );
	let value = wide( value );

	let mut buffer = vec![ 0u16; 1024 ];
	let mut size   = ( buffer.len() * 2 ) as u32;

	let status = unsafe { RegGetValueW( root, key.as_ptr(), value.as_ptr(), RRF_RT_REG_SZ | flags, std::ptr::null_mut(), buffer.as_mut_ptr().cast(), &mut size ) };

	if status != 0
	{
		return None;
	}

	let len  = buffer.iter().position( |&c| c == 0 ).unwrap_or( buffer.len() );
	let text = String::from_utf16_lossy( &buffer[ ..len ] );

	Some( text )
}

fn steam_path() -> Option<PathBuf>
{
	let user    = || registry_string( HKEY_CURRENT_USER, "Software\\Valve\\Steam", "SteamPath", 0 );
	let machine = || registry_string( HKEY_LOCAL_MACHINE, "Software\\Valve\\Steam", "InstallPath", RRF_SUBKEY_WOW6432KEY );

	let path = user().or_else( machine ).map( |path| PathBuf::from( path.replace( '/', "\\" ) ) );

	path
}

// The folder Steam records for INFRA among installed programs
fn registered_infra() -> Option<PathBuf>
{
	let key = format!( "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Steam App {INFRA_APP_ID}" );

	let wow64  = || registry_string( HKEY_LOCAL_MACHINE, &key, "InstallLocation", RRF_SUBKEY_WOW6432KEY );
	let native = || registry_string( HKEY_LOCAL_MACHINE, &key, "InstallLocation", 0 );

	let path = wow64().or_else( native ).map( PathBuf::from );

	path
}

//-----------------------------------------------------------------------------
// Steam libraries
//-----------------------------------------------------------------------------

// The value of a "key" "value" line of a VDF or ACF file
fn vdf_value( line: &str, key: &str ) -> Option<String>
{
	let parts: Vec<&str> = line.split( '"' ).collect();

	if parts.len() < 4 || parts[ 1 ] != key
	{
		return None;
	}

	Some( parts[ 3 ].replace( "\\\\", "\\" ) )
}

fn vdf_values<'a>( text: &'a str, key: &'a str ) -> impl Iterator<Item = String> + 'a
{
	text.lines().filter_map( move |line| vdf_value( line, key ) )
}

// All steamapps folders known to the Steam client
fn libraries() -> Vec<PathBuf>
{
	let Some( steam ) = steam_path() else
	{
		return Vec::new();
	};

	let mut libraries = vec![ steam.join( "steamapps" ) ];

	let vdf = std::fs::read_to_string( steam.join( "steamapps" ).join( "libraryfolders.vdf" ) ).unwrap_or_default();

	for path in vdf_values( &vdf, "path" )
	{
		let library = PathBuf::from( path ).join( "steamapps" );

		if !libraries.contains( &library )
		{
			libraries.push( library );
		}
	}

	libraries
}

// INFRA in a library, by the install folder of its app manifest
fn library_infra( library: PathBuf ) -> PathBuf
{
	let acf         = std::fs::read_to_string( library.join( format!( "appmanifest_{INFRA_APP_ID}.acf" ) ) ).unwrap_or_default();
	let install_dir = vdf_values( &acf, "installdir" ).next().unwrap_or_else( || "infra".to_string() );

	library.join( "common" ).join( install_dir )
}

fn is_infra( dir: &Path ) -> bool
{
	dir.join( "bin" ).join( "hammer.exe" ).is_file() && dir.join( "infra" ).join( "gameinfo.txt" ).is_file()
}

pub fn find_infra() -> Option<PathBuf>
{
	let mut candidates = registered_infra().into_iter()
		.chain( libraries().into_iter().map( library_infra ) );

	let found = candidates.find( |dir| is_infra( dir ) );

	found
}
