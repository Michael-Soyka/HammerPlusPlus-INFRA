//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Video background: assets\background.webp, an animated WebP, played in a loop
//
//=============================================================================//

use crate::gdi::{ self, GdiObject };
use image_webp::{ DecodingError, WebPDecoder };
use std::cell::RefCell;
use std::io::Cursor;
use std::time::{ Duration, Instant };
use windows_sys::Win32::Foundation::{ COLORREF, RECT };
use windows_sys::Win32::Graphics::Gdi::{ BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateSolidBrush, DIB_RGB_COLORS, FillRect, HALFTONE, HDC, SRCCOPY, SetBrushOrgEx, SetStretchBltMode, StretchDIBits };

const VIDEO: &[u8] = include_bytes!( "../assets/background.webp" );

// How often the window asks for a frame: finer than any frame duration
pub const TICK_MS: u32 = 15;

// Black over the frame: the clip is dark already, the text needs a little more
const DIM_ALPHA: u8 = 90;

// A zero duration would stall the loop
const MIN_FRAME_MS: u32 = 10;

// Further behind than this, after a pause, the video goes on from now instead of racing
const MAX_LAG: Duration = Duration::from_millis( 500 );

const BGRA: usize = 4;

struct Player
{
	decoder: WebPDecoder<Cursor<&'static [u8]>>,
	width:   i32,
	height:  i32,

	// RGB or RGBA, as the decoder gives it
	channels: usize,
	decoded:  Vec<u8>,

	// BGRA, as GDI takes it
	pixels: Vec<u8>,

	shown: bool,
	due:   Instant,
}

struct Crop
{
	left:   i32,
	top:    i32,
	width:  i32,
	height: i32,
}

impl Player
{
	fn open() -> Option<Player>
	{
		let decoder = WebPDecoder::new( Cursor::new( VIDEO ) ).ok()?;

		let ( width, height ) = decoder.dimensions();

		let size = decoder.output_buffer_size()?;
		let area = width as usize * height as usize;

		let mut player = Player
		{
			decoder,
			width:    width as i32,
			height:   height as i32,
			channels: size / area.max( 1 ),
			decoded:  vec![ 0; size ],
			pixels:   vec![ 0; area * BGRA ],
			shown:    false,
			due:      Instant::now(),
		};

		if !player.decoder.is_animated()
		{
			player.decoder.read_image( &mut player.decoded ).ok()?;
			player.convert();
			player.shown = true;
		}

		Some( player )
	}

	fn convert( &mut self )
	{
		let ( targets, _ ) = self.pixels.as_chunks_mut::<BGRA>();

		for ( source, target ) in self.decoded.chunks_exact( self.channels ).zip( targets )
		{
			target[ 0 ] = source[ 2 ];
			target[ 1 ] = source[ 1 ];
			target[ 2 ] = source[ 0 ];
			target[ 3 ] = 0xff;
		}
	}

	// From the start again at the end
	fn read_frame( &mut self ) -> Option<u32>
	{
		match self.decoder.read_frame( &mut self.decoded )
		{
			Ok( duration ) => Some( duration ),

			Err( DecodingError::NoMoreFrames ) =>
			{
				self.decoder.reset_animation();

				self.decoder.read_frame( &mut self.decoded ).ok()
			}

			Err( _ ) => None,
		}
	}

	fn is_due( &self, now: Instant ) -> bool
	{
		!self.shown || now >= self.due
	}

	// Decodes the next frame once the shown one has had its time; true when it changed
	fn advance( &mut self ) -> bool
	{
		if !self.decoder.is_animated()
		{
			return false;
		}

		let now = Instant::now();

		if !self.is_due( now )
		{
			return false;
		}

		if now.duration_since( self.due ) > MAX_LAG
		{
			self.due = now;
		}

		let Some( duration ) = self.read_frame() else
		{
			return false;
		};

		self.due += Duration::from_millis( u64::from( duration.max( MIN_FRAME_MS ) ) );

		self.convert();
		self.shown = true;

		true
	}

	// Cropped to the proportions of the target, centered
	fn crop( &self, width: i32, height: i32 ) -> Crop
	{
		let mut crop_width  = self.width;
		let mut crop_height = ( self.width * height ) / width;

		if self.width * height > self.height * width
		{
			crop_width  = ( self.height * width ) / height;
			crop_height = self.height;
		}

		let crop = Crop
		{
			left:   ( self.width - crop_width ) / 2,
			top:    ( self.height - crop_height ) / 2,
			width:  crop_width,
			height: crop_height,
		};

		crop
	}

	fn bitmap_info( &self ) -> BITMAPINFO
	{
		let header = BITMAPINFOHEADER
		{
			biSize:  size_of::<BITMAPINFOHEADER>() as u32,
			biWidth: self.width,

			// Negative: rows go top down, as decoded
			biHeight:      -self.height,
			biPlanes:      1,
			biBitCount:    32,
			biCompression: BI_RGB,
			..Default::default()
		};

		let info = BITMAPINFO { bmiHeader: header, ..Default::default() };

		info
	}

	fn draw( &self, dc: HDC, width: i32, height: i32 )
	{
		let crop = self.crop( width, height );
		let info = self.bitmap_info();

		unsafe
		{
			SetStretchBltMode( dc, HALFTONE );
			SetBrushOrgEx( dc, 0, 0, std::ptr::null_mut() );
			StretchDIBits( dc, 0, 0, width, height, crop.left, crop.top, crop.width, crop.height, self.pixels.as_ptr().cast(), &info, DIB_RGB_COLORS, SRCCOPY );
		}
	}

	// The first frame is decoded on the first draw; false while there is none
	fn draw_current( &mut self, dc: HDC, width: i32, height: i32 ) -> bool
	{
		if !self.shown
		{
			self.advance();
		}

		self.draw( dc, width, height );

		self.shown
	}
}

thread_local!
{
	static PLAYER: RefCell<Option<Player>> = RefCell::new( Player::open() );
}

// True when a new frame is due to be drawn
pub fn advance() -> bool
{
	PLAYER.with_borrow_mut( |player| player.as_mut().is_some_and( Player::advance ) )
}

fn fill( dc: HDC, width: i32, height: i32, color: COLORREF )
{
	let brush = GdiObject( unsafe { CreateSolidBrush( color ) } );
	let rect  = RECT { left: 0, top: 0, right: width, bottom: height };

	unsafe { FillRect( dc, &rect, brush.0 ) };
}

// The current frame over width x height, dimmed; fallback when the video does not decode
pub fn draw( dc: HDC, width: i32, height: i32, fallback: COLORREF )
{
	let drawn = PLAYER.with_borrow_mut( |player| player.as_mut().is_some_and( |player| player.draw_current( dc, width, height ) ) );

	if drawn
	{
		gdi::dim( dc, width, height, DIM_ALPHA );

		return;
	}

	fill( dc, width, height, fallback );
}
