//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: SVG icons of the buttons, rendered once for every pixel size
//
//=============================================================================//

use crate::gdiplus::{ self, Bitmap, Graphics };
use resvg::tiny_skia::{ Pixmap, Transform };
use resvg::usvg::{ Options, Tree };
use std::cell::RefCell;
use std::collections::HashMap;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::HDC;
use windows_sys::Win32::Graphics::GdiPlus::GdipDrawImageRectI;

#[ derive( Clone, Copy, PartialEq, Eq, Hash ) ]
pub enum Icon
{
	Download,
	Info,
}

impl Icon
{
	fn svg( self ) -> &'static [u8]
	{
		match self
		{
			Icon::Download => include_bytes!( "../assets/main/icon-download.svg" ),
			Icon::Info => include_bytes!( "../assets/main/icon-info.svg" ),
		}
	}
}

thread_local!
{
	static RENDERED: RefCell<HashMap<(Icon, i32), Option<Bitmap>>> = RefCell::new( HashMap::new() );
}

// Before GDI+ shuts down
pub fn clear()
{
	RENDERED.with_borrow_mut( HashMap::clear );
}

fn render( icon: Icon, size: i32 ) -> Option<Bitmap>
{
	let tree       = Tree::from_data( icon.svg(), &Options::default() ).ok()?;
	let mut pixmap = Pixmap::new( size as u32, size as u32 )?;
	let scale      = size as f32 / tree.size().width();

	resvg::render( &tree, Transform::from_scale( scale, scale ), &mut pixmap.as_mut() );

	// tiny-skia keeps premultiplied RGBA, GDI+ reads premultiplied BGRA
	let mut pixels = pixmap.take();

	for pixel in pixels.as_chunks_mut::<4>().0
	{
		pixel.swap( 0, 2 );
	}

	gdiplus::from_pixels( size, size, &pixels )
}

// Size pixels square, centered in rect
pub fn draw( dc: HDC, rect: &RECT, icon: Icon, size: i32 )
{
	let x = ( rect.left + rect.right - size ) / 2;
	let y = ( rect.top + rect.bottom - size ) / 2;

	RENDERED.with_borrow_mut(
		|rendered|
		{
			let Some( bitmap ) = rendered.entry( ( icon, size ) ).or_insert_with( || render( icon, size ) ) else
			{
				return;
			};

			let graphics = Graphics::from_dc( dc );

			unsafe { GdipDrawImageRectI( graphics.0, bitmap.0.cast(), x, y, size, size ) };
		}
	);
}
