//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Running copies of an executable: finds them by the full image path and asks them to close
//
//=============================================================================//

use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;
use windows_sys::Win32::Foundation::{ CloseHandle, HWND, INVALID_HANDLE_VALUE, LPARAM };
use windows_sys::Win32::System::Diagnostics::ToolHelp::{ CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS };
use windows_sys::Win32::System::Threading::{ OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW };
use windows_sys::Win32::UI::WindowsAndMessaging::{ EnumWindows, GW_OWNER, GetWindow, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, WM_CLOSE };

// Long enough for any image path
const PATH_LENGTH: usize = 32768;

// After close(): how often to look and how long to wait for the processes to exit
pub const CLOSE_POLL_MS: u32 = 100;
pub const CLOSE_TIMEOUT: Duration = Duration::from_secs( 30 );

fn from_wide( buffer: &[u16] ) -> String
{
	let length = buffer.iter().position( |&c| c == 0 ).unwrap_or( buffer.len() );

	String::from_utf16_lossy( &buffer[ ..length ] )
}

// None when the process does not let its path be read
fn image_path( pid: u32 ) -> Option<String>
{
	let process = unsafe { OpenProcess( PROCESS_QUERY_LIMITED_INFORMATION, 0, pid ) };

	if process.is_null()
	{
		return None;
	}

	let mut buffer = vec![ 0u16; PATH_LENGTH ];
	let mut length = PATH_LENGTH as u32;

	let read = unsafe { QueryFullProcessImageNameW( process, PROCESS_NAME_WIN32, buffer.as_mut_ptr(), &mut length ) } != 0;

	unsafe { CloseHandle( process ) };

	let path = read.then( || from_wide( &buffer[ ..length as usize ] ) );

	path
}

// Ids and file names of all processes
fn processes() -> Vec<(u32, String)>
{
	let snapshot = unsafe { CreateToolhelp32Snapshot( TH32CS_SNAPPROCESS, 0 ) };

	if snapshot == INVALID_HANDLE_VALUE
	{
		return Vec::new();
	}

	let mut found = Vec::new();
	let mut entry = PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };

	let mut more = unsafe { Process32FirstW( snapshot, &mut entry ) } != 0;

	while more
	{
		found.push( ( entry.th32ProcessID, from_wide( &entry.szExeFile ) ) );

		more = unsafe { Process32NextW( snapshot, &mut entry ) } != 0;
	}

	unsafe { CloseHandle( snapshot ) };

	found
}

// Processes started from exactly this file, not from another copy with the same name
pub fn running( exe: &Path ) -> Vec<u32>
{
	let Ok( exe ) = std::path::absolute( exe ) else
	{
		return Vec::new();
	};

	let path = exe.to_string_lossy().to_lowercase();
	let name = exe.file_name().map( OsStr::to_string_lossy ).unwrap_or_default().to_lowercase();

	let found = processes().into_iter()
		.filter( |( _, file )| file.to_lowercase() == name )
		.map( |( pid, _ )| pid )
		.filter( |&pid| image_path( pid ).is_some_and( |image| image.to_lowercase() == path ) )
		.collect();

	found
}

// Main windows only: owned ones are dialogs of the main window
unsafe extern "system" fn close_window( window: HWND, pids: LPARAM ) -> i32
{
	let pids = unsafe { *( pids as *const &[u32] ) };

	let mut pid = 0;

	unsafe { GetWindowThreadProcessId( window, &mut pid ) };

	let is_main = unsafe { IsWindowVisible( window ) } != 0 && unsafe { GetWindow( window, GW_OWNER ) }.is_null();

	if is_main && pids.contains( &pid )
	{
		unsafe { PostMessageW( window, WM_CLOSE, 0, 0 ) };
	}

	1
}

// As the close button does: the application may ask to save and stays open until answered
pub fn close( pids: &[u32] )
{
	// A slice reference is two words: the callback gets a pointer to it
	unsafe { EnumWindows( Some( close_window ), ( &raw const pids ) as LPARAM ) };
}
