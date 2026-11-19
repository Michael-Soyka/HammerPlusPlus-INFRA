//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: The Windows 10 functions of the window, found at run time: imported, they would stop
//			the exe from loading on Windows 8 and 8.1, where the command line still works
//
//=============================================================================//

use std::ffi::CStr;
use windows_sys::Win32::Foundation::{ HMODULE, RECT };
use windows_sys::Win32::System::LibraryLoader::{ GetModuleHandleW, GetProcAddress };
use windows_sys::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT;
use windows_sys::Win32::UI::WindowsAndMessaging::{ SYSTEM_METRICS_INDEX, WINDOW_EX_STYLE, WINDOW_STYLE };
use windows_sys::core::BOOL;

type SetProcessDpiAwarenessContextFn = unsafe extern "system" fn( DPI_AWARENESS_CONTEXT ) -> BOOL;
type GetDpiForSystemFn               = unsafe extern "system" fn() -> u32;
type AdjustWindowRectExForDpiFn      = unsafe extern "system" fn( *mut RECT, WINDOW_STYLE, BOOL, WINDOW_EX_STYLE, u32 ) -> BOOL;
type GetSystemMetricsForDpiFn        = unsafe extern "system" fn( SYSTEM_METRICS_INDEX, u32 ) -> i32;

pub struct Win10
{
	set_process_dpi_awareness_context: SetProcessDpiAwarenessContextFn,
	get_dpi_for_system:                GetDpiForSystemFn,
	adjust_window_rect_ex_for_dpi:     AdjustWindowRectExForDpiFn,
	get_system_metrics_for_dpi:        GetSystemMetricsForDpiFn,
}

// GetProcAddress gives an untyped pointer: the type comes from the documented signature
fn find<T: Copy>( module: HMODULE, name: &CStr ) -> Option<T>
{
	const { assert!( size_of::<T>() == size_of::<usize>() ) };

	let function = unsafe { GetProcAddress( module, name.as_ptr().cast() ) }?;

	Some( unsafe { std::mem::transmute_copy( &function ) } )
}

impl Win10
{
	// None before Windows 10 1607
	pub fn load() -> Option<Win10>
	{
		let user32 = unsafe { GetModuleHandleW( crate::ui::wide( "user32.dll" ).as_ptr() ) };

		if user32.is_null()
		{
			return None;
		}

		let functions = Win10
		{
			set_process_dpi_awareness_context: find( user32, c"SetProcessDpiAwarenessContext" )?,
			get_dpi_for_system:                find( user32, c"GetDpiForSystem" )?,
			adjust_window_rect_ex_for_dpi:     find( user32, c"AdjustWindowRectExForDpi" )?,
			get_system_metrics_for_dpi:        find( user32, c"GetSystemMetricsForDpi" )?,
		};

		Some( functions )
	}

	pub fn set_process_dpi_awareness_context( &self, context: DPI_AWARENESS_CONTEXT )
	{
		unsafe { ( self.set_process_dpi_awareness_context )( context ) };
	}

	pub fn get_dpi_for_system( &self ) -> u32
	{
		unsafe { ( self.get_dpi_for_system )() }
	}

	pub fn adjust_window_rect_ex_for_dpi( &self, rect: &mut RECT, style: WINDOW_STYLE, ex_style: WINDOW_EX_STYLE, dpi: u32 )
	{
		unsafe { ( self.adjust_window_rect_ex_for_dpi )( rect, style, 0, ex_style, dpi ) };
	}

	pub fn get_system_metrics_for_dpi( &self, index: SYSTEM_METRICS_INDEX, dpi: u32 ) -> i32
	{
		unsafe { ( self.get_system_metrics_for_dpi )( index, dpi ) }
	}
}
