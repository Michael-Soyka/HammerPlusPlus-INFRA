//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Embeds the release adapter, the app icon and the Common Controls 6 manifest
//			into the installer
//
//=============================================================================//

use std::path::PathBuf;

const COMMON_CONTROLS: &str = "type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'";

fn target_dir() -> PathBuf
{
	let manifest_dir = PathBuf::from( std::env::var_os( "CARGO_MANIFEST_DIR" ).unwrap_or_default() );
	let workspace    = manifest_dir.join( ".." ).join( ".." );

	std::env::var_os( "CARGO_TARGET_DIR" ).map( PathBuf::from ).unwrap_or( workspace.join( "target" ) )
}

fn embed_adapter()
{
	let adapter = target_dir().join( "i686-pc-windows-msvc" ).join( "release" ).join( "infra_hpp_adapter.dll" );

	if !adapter.is_file()
	{
		panic!( "{} is missing, build it first: cargo build --release -p infra_hpp_adapter --target i686-pc-windows-msvc", adapter.display() );
	}

	println!( "cargo:rerun-if-changed={}", adapter.display() );
	println!( "cargo:rustc-env=INFRA_HPP_ADAPTER={}", adapter.display() );
}

fn embed_icon()
{
	println!( "cargo:rerun-if-changed=app.rc" );
	println!( "cargo:rerun-if-changed=assets/hpp-patcher-infra-app-icon.ico" );

	embed_resource::compile( "app.rc", embed_resource::NONE ).manifest_required().unwrap_or_else( |error| panic!( "app.rc: {error}" ) );
}

fn embed_common_controls()
{
	if !std::env::var( "CARGO_CFG_TARGET_ENV" ).is_ok_and( |env| env == "msvc" )
	{
		return;
	}

	println!( "cargo:rustc-link-arg-bins=/MANIFEST:EMBED" );
	println!( "cargo:rustc-link-arg-bins=/MANIFESTDEPENDENCY:{COMMON_CONTROLS}" );
}

fn main()
{
	embed_adapter();
	embed_icon();
	embed_common_controls();
}
