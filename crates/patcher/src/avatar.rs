//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: GitHub avatars: embedded ones at once, downloaded ones over them,
//			drawn as smooth circles
//
//=============================================================================//

use crate::gdiplus::{ self, Bitmap, Brush, Graphics };
use crate::ui::wide;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::ptr::{ null, null_mut };
use windows_sys::Win32::Foundation::{ COLORREF, HWND, RECT };
use windows_sys::Win32::Graphics::Gdi::HDC;
use windows_sys::Win32::Graphics::GdiPlus::{ GdipCreateTexture, GdipDrawImageRectI, GdipFillEllipseI, GdipSetInterpolationMode, GdipSetSmoothingMode, GdipTranslateTextureTransform, InterpolationModeHighQualityBicubic, MatrixOrderAppend, SmoothingModeAntiAlias, WrapModeClamp };
use windows_sys::Win32::System::Com::Urlmon::URLDownloadToFileW;
use windows_sys::Win32::System::Com::{ COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize };
use windows_sys::Win32::UI::WindowsAndMessaging::{ PostMessageW, WM_APP };

// Posted to the window passed to download once the downloads are over
pub const WM_AVATARS: u32 = WM_APP + 1;

// Downloaded larger than drawn: scaled down on load for any DPI
const DOWNLOAD_SIZE: i32 = 160;

thread_local!
{
	static AVATARS: RefCell<HashMap<String, Bitmap>> = RefCell::new( HashMap::new() );
}

// Before GDI+ shuts down
pub fn clear()
{
	AVATARS.with_borrow_mut( HashMap::clear );
}

fn file_path( login: &str ) -> PathBuf
{
	std::env::temp_dir().join( format!( "infra-hpp-avatar-{}-{login}.png", std::process::id() ) )
}

//-----------------------------------------------------------------------------
// Download
//-----------------------------------------------------------------------------

fn download_file( login: &str )
{
	let url  = wide( format!( "https://github.com/{login}.png?size={DOWNLOAD_SIZE}" ) );
	let path = wide( file_path( login ) );

	unsafe { URLDownloadToFileW( null_mut(), url.as_ptr(), path.as_ptr(), 0, null_mut() ) };
}

// HWND is a raw pointer, which threads do not share, so it travels as an integer
fn download_all( window: isize, logins: Vec<&'static str> )
{
	unsafe { CoInitializeEx( null(), COINIT_MULTITHREADED as u32 ) };

	for login in logins
	{
		download_file( login );
	}

	unsafe
	{
		CoUninitialize();
		PostMessageW( window as HWND, WM_AVATARS, 0, 0 );
	}
}

// Without internet the files just do not appear, and the embedded avatars stay
pub fn download( window: HWND, logins: Vec<&'static str> )
{
	let window = window as isize;

	std::thread::spawn( move || download_all( window, logins ) );
}

//-----------------------------------------------------------------------------
// Loading
//-----------------------------------------------------------------------------

fn scaled( source: &Bitmap, size: i32 ) -> Option<Bitmap>
{
	let scaled   = gdiplus::blank( size, size )?;
	let graphics = Graphics::from_bitmap( &scaled );

	unsafe
	{
		GdipSetInterpolationMode( graphics.0, InterpolationModeHighQualityBicubic );
		GdipDrawImageRectI( graphics.0, source.0.cast(), 0, 0, size, size );
	}

	Some( scaled )
}

fn insert( login: &str, source: Option<Bitmap>, size: i32 )
{
	let Some( bitmap ) = source.and_then( |source| scaled( &source, size ) ) else
	{
		return;
	};

	AVATARS.with_borrow_mut( |avatars| avatars.insert( login.to_string(), bitmap ) );
}

// The avatars shipped in the installer, scaled to size pixels; downloaded ones replace them
pub fn load_embedded( avatars: &[(&str, &[u8])], size: i32 )
{
	for ( login, data ) in avatars
	{
		insert( login, gdiplus::decode( data ), size );
	}
}

// Takes the downloaded files, scaled to size pixels, and deletes them
pub fn load_downloaded( logins: &[&str], size: i32 )
{
	for login in logins
	{
		let path = file_path( login );

		insert( login, gdiplus::load_file( &path ), size );

		let _ = std::fs::remove_file( &path );
	}
}

//-----------------------------------------------------------------------------
// Drawing
//-----------------------------------------------------------------------------

fn fill_with_avatar( graphics: &Graphics, bitmap: &Bitmap, rect: &RECT )
{
	let mut texture = null_mut();

	unsafe { GdipCreateTexture( bitmap.0.cast(), WrapModeClamp, &mut texture ) };

	let brush = Brush( texture.cast() );

	unsafe
	{
		GdipTranslateTextureTransform( texture, rect.left as f32, rect.top as f32, MatrixOrderAppend );
		GdipFillEllipseI( graphics.0, brush.0, rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top );
	}
}

fn fill_with_color( graphics: &Graphics, color: COLORREF, rect: &RECT )
{
	let brush = Brush::solid( gdiplus::argb( 0xff, color ) );

	unsafe { GdipFillEllipseI( graphics.0, brush.0, rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top ) };
}

fn fill_with_stored( graphics: &Graphics, login: &str, rect: &RECT ) -> bool
{
	AVATARS.with_borrow(
		|avatars|
		{
			let Some( bitmap ) = avatars.get( login ) else
			{
				return false;
			};

			fill_with_avatar( graphics, bitmap, rect );

			true
		}
	)
}

// A circle with the avatar of login, or with color while there is none; true when the avatar is drawn
pub fn draw_circle( dc: HDC, rect: &RECT, login: &str, color: COLORREF ) -> bool
{
	let graphics = Graphics::from_dc( dc );

	unsafe { GdipSetSmoothingMode( graphics.0, SmoothingModeAntiAlias ) };

	let drawn = fill_with_stored( &graphics, login, rect );

	if !drawn
	{
		fill_with_color( &graphics, color, rect );
	}

	drawn
}
