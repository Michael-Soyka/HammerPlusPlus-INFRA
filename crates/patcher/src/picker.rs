//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Folder picker: the Explorer dialog with the address bar, in place of the old folder tree
//
//=============================================================================//

use std::path::{ Path, PathBuf };
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{ CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree };
use windows::Win32::UI::Shell::{ FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, FileOpenDialog, IFileOpenDialog, IShellItem, SHCreateItemFromParsingName, SIGDN_FILESYSPATH };
use windows::core::HSTRING;

// The folder of the field, or its nearest existing parent
fn start_folder( current: &Path ) -> Option<IShellItem>
{
	let folder = current.ancestors().find( |dir| dir.is_dir() )?;

	unsafe { SHCreateItemFromParsingName( &HSTRING::from( folder ), None ) }.ok()
}

fn item_path( item: &IShellItem ) -> windows::core::Result<PathBuf>
{
	let name = unsafe { item.GetDisplayName( SIGDN_FILESYSPATH ) }?;
	let text = unsafe { name.to_string() };

	unsafe { CoTaskMemFree( Some( name.0.cast_const().cast() ) ) };

	let path = text.map( PathBuf::from ).map_err( |_| windows::core::Error::empty() )?;

	Ok( path )
}

// Cancel comes back as an error too
fn show( owner: HWND, title: &str, current: &Path ) -> windows::core::Result<PathBuf>
{
	let dialog: IFileOpenDialog = unsafe { CoCreateInstance( &FileOpenDialog, None, CLSCTX_INPROC_SERVER ) }?;

	unsafe
	{
		let options = dialog.GetOptions()?;

		dialog.SetOptions( options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM )?;
		dialog.SetTitle( &HSTRING::from( title ) )?;

		if let Some( folder ) = start_folder( current )
		{
			dialog.SetFolder( &folder )?;
		}

		dialog.Show( Some( owner ) )?;
	}

	let item = unsafe { dialog.GetResult() }?;

	item_path( &item )
}

// None when cancelled
pub fn pick( owner: windows_sys::Win32::Foundation::HWND, title: &str, current: &Path ) -> Option<PathBuf>
{
	show( HWND( owner ), title, current ).ok()
}
