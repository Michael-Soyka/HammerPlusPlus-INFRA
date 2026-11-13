//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Authors card: slides up from the bottom over a dimmed snapshot of the installer window,
//			with avatars from GitHub and links to the authors' pages
//
//=============================================================================//

use crate::avatar;
use crate::gdi::{ self, Canvas };
use crate::gdiplus::{ self, Graphics, Pen };
use crate::palette::{ ACCENT, BORDER, BUTTON_TEXT, DIM, FIELD, GOOD, TEXT, rgb };
use crate::ui::{ self, scale, wide };
use std::cell::RefCell;
use std::ptr::{ null, null_mut };
use std::time::{ Duration, Instant };
use windows_sys::Win32::Foundation::{ COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM };
use windows_sys::Win32::Graphics::Gdi::{ BeginPaint, BitBlt, DT_BOTTOM, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_SINGLELINE, DT_TOP, DT_VCENTER, EndPaint, FW_NORMAL, FW_SEMIBOLD, GetDC, HDC, InvalidateRect, PAINTSTRUCT, RDW_ALLCHILDREN, RDW_UPDATENOW, RedrawWindow, ReleaseDC, SRCCOPY, ScreenToClient };
use windows_sys::Win32::Graphics::GdiPlus::{ GdipDrawLineI, GdipSetSmoothingMode, SmoothingModeAntiAlias };
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::WM_MOUSELEAVE;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{ GetFocus, SetFocus, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, VK_ESCAPE };
use windows_sys::Win32::UI::WindowsAndMessaging::{ CreateWindowExW, DLGC_WANTALLKEYS, DefWindowProcW, DestroyWindow, GetClientRect, GetCursorPos, HWND_TOP, IDC_ARROW, IDC_HAND, KillTimer, LoadCursorW, RegisterClassExW, SWP_NOMOVE, SWP_NOSIZE, SetCursor, SetTimer, SetWindowPos, WM_ERASEBKGND, WM_GETDLGCODE, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_PAINT, WM_SETCURSOR, WM_TIMER, WNDCLASSEXW, WS_CHILD, WS_VISIBLE };

struct Author
{
	// GitHub login, for the avatar
	login: &'static str,

	// Embedded, for working without internet
	avatar: &'static [u8],

	// False keeps the embedded avatar even with internet
	download: bool,

	name:  &'static str,
	role:  &'static str,
	color: COLORREF,

	// Opened by the link icon
	url: &'static str,
}

const AUTHORS: [Author; 3] =
[
	Author
	{
		login:    "loiste-interactive",
		avatar:   include_bytes!( "../assets/loiste.png" ),
		download: false,
		name:     "Loiste Interactive",
		role:     "Creators of INFRA",
		color:    rgb( 200, 120, 60 ),
		url:      "https://loisteinteractive.com",
	},

	Author
	{
		login:    "ficool2",
		avatar:   include_bytes!( "../assets/34815548.png" ),
		download: true,
		name:     "Ficool2",
		role:     "Creator of Hammer++",
		color:    ACCENT,
		url:      "https://github.com/ficool2",
	},

	Author
	{
		login:    "Michael-Soyka",
		avatar:   include_bytes!( "../assets/45078459.png" ),
		download: true,
		name:     "Michael Soyka (Moon-6)",
		role:     "Structure, project direction, tests",
		color:    GOOD,
		url:      "https://github.com/Michael-Soyka",
	},
];

const CLASS: &str = "InfraHppAuthors";

// At 96 DPI
const CARD_WIDTH:   i32 = 440;
const CARD_PADDING: i32 = 20;
const CARD_RADIUS:  i32 = 16;
const AVATAR:       i32 = 40;
const ROW_GAP:      i32 = 18;
const TEXT_GAP:     i32 = 14;
const ROLE_GAP:     i32 = 2;
const LINK:         i32 = 14;

// Around the link icon, still a click on it
const LINK_HIT: i32 = 8;

// Between the text and the link icon
const LINK_GAP: i32 = 12;

// The link icon is drawn on a 14x14 grid with a 1.5 px pen at 96 DPI
const LINK_GRID:       i32 = 14;
const LINK_PEN_TENTHS: i32 = 15;

const DIM_ALPHA: f32 = 160.0;
const ANIMATION: Duration = Duration::from_millis( 240 );
const FRAME_MS:  u32 = 10;
const TIMER_ID:  usize = 1;

// 0 is hidden below the window, 1 is open
const HIDDEN: f32 = 0.0;
const OPEN:   f32 = 1.0;

struct Fonts
{
	name:   gdi::GdiObject,
	role:   gdi::GdiObject,
	avatar: gdi::GdiObject,
}

struct Modal
{
	overlay:  HWND,
	snapshot: Canvas,
	fonts:    Fonts,
	width:    i32,
	height:   i32,

	// Gets the focus back on closing
	focus: HWND,

	from:  f32,
	to:    f32,
	start: Instant,

	// Index of the author whose link is under the cursor
	link: Option<usize>,
}

thread_local!
{
	static MODAL: RefCell<Option<Modal>> = const { RefCell::new( None ) };
}

fn with_modal<R>( read: impl FnOnce(&Modal) -> R ) -> Option<R>
{
	MODAL.with_borrow( |slot| slot.as_ref().map( read ) )
}

fn with_modal_mut<R>( update: impl FnOnce(&mut Modal) -> R ) -> Option<R>
{
	MODAL.with_borrow_mut( |slot| slot.as_mut().map( update ) )
}

//-----------------------------------------------------------------------------
// Geometry
//-----------------------------------------------------------------------------

fn card_height() -> i32
{
	let rows = AUTHORS.len() as i32;

	scale( ( 2 * CARD_PADDING ) + ( rows * AVATAR ) + ( ( rows - 1 ) * ROW_GAP ) )
}

fn row_top( card: &RECT, index: usize ) -> i32
{
	card.top + scale( CARD_PADDING ) + ( index as i32 * scale( AVATAR + ROW_GAP ) )
}

// Right in the row, centered on the avatar
fn link_rect( card: &RECT, index: usize ) -> RECT
{
	let size  = scale( LINK );
	let right = card.right - scale( CARD_PADDING );
	let top   = row_top( card, index ) + ( ( scale( AVATAR ) - size ) / 2 );

	let link = RECT { left: right - size, top, right, bottom: top + size };

	link
}

fn contains( rect: &RECT, point: POINT ) -> bool
{
	point.x >= rect.left && point.x < rect.right && point.y >= rect.top && point.y < rect.bottom
}

fn is_link_hit( card: &RECT, index: usize, point: POINT ) -> bool
{
	let hit  = scale( LINK_HIT );
	let link = link_rect( card, index );
	let area = RECT { left: link.left - hit, top: link.top - hit, right: link.right + hit, bottom: link.bottom + hit };

	contains( &area, point )
}

impl Modal
{
	// Eased out
	fn value( &self ) -> f32
	{
		let progress = ( self.start.elapsed().as_secs_f32() / ANIMATION.as_secs_f32() ).min( 1.0 );
		let eased    = 1.0 - ( 1.0 - progress ).powi( 3 );

		self.from + ( ( self.to - self.from ) * eased )
	}

	fn is_animation_over( &self ) -> bool
	{
		self.start.elapsed() >= ANIMATION
	}

	fn card( &self, value: f32 ) -> RECT
	{
		let width    = scale( CARD_WIDTH );
		let height   = card_height();
		let left     = ( self.width - width ) / 2;
		let open_top = ( self.height - height ) / 2;
		let top      = open_top + ( ( 1.0 - value ) * ( self.height - open_top ) as f32 ) as i32;

		let card = RECT { left, top, right: left + width, bottom: top + height };

		card
	}

	fn current_card( &self ) -> RECT
	{
		self.card( self.value() )
	}

	fn link_at( &self, point: POINT ) -> Option<usize>
	{
		let card = self.current_card();

		let found = ( 0..AUTHORS.len() ).find( |&index| is_link_hit( &card, index, point ) );

		found
	}

	// False when it is already closing
	fn start_closing( &mut self ) -> bool
	{
		if self.to == HIDDEN
		{
			return false;
		}

		self.from  = self.value();
		self.to    = HIDDEN;
		self.start = Instant::now();

		true
	}

	// True when the hovered link changed
	fn set_link( &mut self, link: Option<usize> ) -> bool
	{
		std::mem::replace( &mut self.link, link ) != link
	}
}

//-----------------------------------------------------------------------------
// Avatars
//-----------------------------------------------------------------------------

fn downloaded_logins() -> Vec<&'static str>
{
	AUTHORS.iter().filter( |author| author.download ).map( |author| author.login ).collect()
}

// The embedded avatars right away, then the GitHub ones over them when they download
pub fn load_avatars( window: HWND )
{
	let embedded: Vec<(&str, &[u8])> = AUTHORS.iter().map( |author| ( author.login, author.avatar ) ).collect();

	avatar::load_embedded( &embedded, scale( AVATAR ) );
	avatar::download( window, downloaded_logins() );
}

// Repaints an open card with the avatars
pub fn on_avatars()
{
	avatar::load_downloaded( &downloaded_logins(), scale( AVATAR ) );

	if let Some( overlay ) = with_modal( |modal| modal.overlay )
	{
		unsafe { InvalidateRect( overlay, null(), 0 ) };
	}
}

//-----------------------------------------------------------------------------
// Opening and closing
//-----------------------------------------------------------------------------

pub fn register_class()
{
	let class = wide( CLASS );

	let window_class = WNDCLASSEXW
	{
		cbSize:        size_of::<WNDCLASSEXW>() as u32,
		lpfnWndProc:   Some( overlay_proc ),
		hInstance:     unsafe { GetModuleHandleW( null() ) },
		hCursor:       unsafe { LoadCursorW( null_mut(), IDC_ARROW ) },
		lpszClassName: class.as_ptr(),
		..Default::default()
	};

	unsafe { RegisterClassExW( &window_class ) };
}

fn create_fonts() -> Fonts
{
	let fonts = Fonts
	{
		name:   gdi::GdiObject( gdi::create_font( 15, FW_SEMIBOLD as i32 ) ),
		role:   gdi::GdiObject( gdi::create_font( 14, FW_NORMAL as i32 ) ),
		avatar: gdi::GdiObject( gdi::create_font( 18, FW_SEMIBOLD as i32 ) ),
	};

	fonts
}

// The client area as it looks now: the backdrop under the card
fn take_snapshot( window: HWND, width: i32, height: i32 ) -> Canvas
{
	// Pending paints first, or the snapshot catches controls half drawn
	unsafe { RedrawWindow( window, null(), null_mut(), RDW_UPDATENOW | RDW_ALLCHILDREN ) };

	let window_dc = unsafe { GetDC( window ) };
	let snapshot  = Canvas::new( window_dc, width, height );

	unsafe
	{
		BitBlt( snapshot.dc, 0, 0, width, height, window_dc, 0, 0, SRCCOPY );
		ReleaseDC( window, window_dc );
	}

	snapshot
}

fn create_overlay( window: HWND, width: i32, height: i32 ) -> HWND
{
	let class = wide( CLASS );

	unsafe { CreateWindowExW( 0, class.as_ptr(), null(), WS_CHILD | WS_VISIBLE, 0, 0, width, height, window, null_mut(), GetModuleHandleW( null() ), null() ) }
}

pub fn is_open() -> bool
{
	MODAL.with_borrow( Option::is_some )
}

pub fn open()
{
	if is_open()
	{
		return;
	}

	let window = ui::window();

	let mut client = RECT::default();

	unsafe { GetClientRect( window, &mut client ) };

	let width    = client.right;
	let height   = client.bottom;
	let snapshot = take_snapshot( window, width, height );
	let focus    = unsafe { GetFocus() };
	let overlay  = create_overlay( window, width, height );

	if overlay.is_null()
	{
		return;
	}

	let modal = Modal
	{
		overlay,
		snapshot,
		fonts: create_fonts(),
		width,
		height,
		focus,
		from:  HIDDEN,
		to:    OPEN,
		start: Instant::now(),
		link:  None,
	};

	MODAL.with_borrow_mut( |slot| *slot = Some( modal ) );

	unsafe
	{
		SetWindowPos( overlay, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE );
		SetFocus( overlay );
		SetTimer( overlay, TIMER_ID, FRAME_MS, None );
	}
}

// Slides the card back down from wherever it is now
fn close()
{
	let closing = with_modal_mut( Modal::start_closing ).unwrap_or( false );

	if !closing
	{
		return;
	}

	if let Some( overlay ) = with_modal( |modal| modal.overlay )
	{
		unsafe { SetTimer( overlay, TIMER_ID, FRAME_MS, None ) };
	}
}

fn finish_close()
{
	let Some( modal ) = MODAL.with_borrow_mut( Option::take ) else
	{
		return;
	};

	unsafe
	{
		DestroyWindow( modal.overlay );
		SetFocus( modal.focus );
	}
}

//-----------------------------------------------------------------------------
// Drawing
//-----------------------------------------------------------------------------

// A box open at the top right with an arrow leaving it, on the link grid
const LINK_LINES: [((i32, i32), (i32, i32)); 7] =
[
	( ( 0, 3 ), ( 0, 14 ) ),
	( ( 0, 14 ), ( 11, 14 ) ),
	( ( 11, 14 ), ( 11, 8 ) ),
	( ( 0, 3 ), ( 6, 3 ) ),
	( ( 5, 9 ), ( 14, 0 ) ),
	( ( 8, 0 ), ( 14, 0 ) ),
	( ( 14, 0 ), ( 14, 6 ) ),
];

fn grid_point( rect: &RECT, ( x, y ): (i32, i32) ) -> (i32, i32)
{
	let size = rect.right - rect.left;

	( rect.left + ( ( x * size ) / LINK_GRID ), rect.top + ( ( y * size ) / LINK_GRID ) )
}

fn draw_link( dc: HDC, rect: &RECT, color: COLORREF )
{
	let graphics = Graphics::from_dc( dc );
	let pen      = Pen::round( gdiplus::argb( 0xff, color ), ( scale( LINK_PEN_TENTHS ) as f32 ) / 10.0 );

	unsafe { GdipSetSmoothingMode( graphics.0, SmoothingModeAntiAlias ) };

	for ( from, to ) in LINK_LINES
	{
		let ( from_x, from_y ) = grid_point( rect, from );
		let ( to_x, to_y ) = grid_point( rect, to );

		unsafe { GdipDrawLineI( graphics.0, pen.0, from_x, from_y, to_x, to_y ) };
	}
}

// The avatar, or the first letter of the name in a circle without one
fn draw_avatar( dc: HDC, author: &Author, fonts: &Fonts, avatar: &RECT )
{
	if avatar::draw_circle( dc, avatar, author.login, author.color )
	{
		return;
	}

	let initial = author.name.chars().next().map( String::from ).unwrap_or_default();

	gdi::draw_text( dc, &initial, *avatar, fonts.avatar.0, BUTTON_TEXT, DT_CENTER | DT_VCENTER | DT_SINGLELINE );
}

// The avatar, then the name over the role
fn draw_author( dc: HDC, author: &Author, fonts: &Fonts, left: i32, top: i32, right: i32 )
{
	let size   = scale( AVATAR );
	let avatar = RECT { left, top, right: left + size, bottom: top + size };

	draw_avatar( dc, author, fonts, &avatar );

	let text_left = avatar.right + scale( TEXT_GAP );
	let middle    = top + ( size / 2 );
	let line      = DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS;
	let name      = RECT { left: text_left, top, right, bottom: middle };
	let role      = RECT { left: text_left, top: middle + scale( ROLE_GAP ), right, bottom: avatar.bottom };

	gdi::draw_text( dc, author.name, name, fonts.name.0, TEXT, line | DT_BOTTOM );
	gdi::draw_text( dc, author.role, role, fonts.role.0, DIM, line | DT_TOP );
}

fn draw_card( dc: HDC, card: &RECT, fonts: &Fonts, hovered: Option<usize> )
{
	gdi::fill_round_rect( dc, card, FIELD, BORDER, CARD_RADIUS );

	let left = card.left + scale( CARD_PADDING );

	for ( index, author ) in AUTHORS.iter().enumerate()
	{
		let link = link_rect( card, index );

		draw_author( dc, author, fonts, left, row_top( card, index ), link.left - scale( LINK_GAP ) );

		let mut color = DIM;

		if hovered == Some( index )
		{
			color = TEXT;
		}

		draw_link( dc, &link, color );
	}
}

// Composed off screen, so a frame never flickers
fn draw_frame( dc: HDC, modal: &Modal )
{
	let value  = modal.value();
	let width  = modal.width;
	let height = modal.height;
	let frame  = Canvas::new( dc, width, height );

	unsafe { BitBlt( frame.dc, 0, 0, width, height, modal.snapshot.dc, 0, 0, SRCCOPY ) };

	gdi::dim( frame.dc, width, height, ( value * DIM_ALPHA ) as u8 );
	draw_card( frame.dc, &modal.card( value ), &modal.fonts, modal.link );

	unsafe { BitBlt( dc, 0, 0, width, height, frame.dc, 0, 0, SRCCOPY ) };
}

//-----------------------------------------------------------------------------
// Events
//-----------------------------------------------------------------------------

fn on_paint( overlay: HWND )
{
	let mut paint = PAINTSTRUCT::default();

	let dc = unsafe { BeginPaint( overlay, &mut paint ) };

	with_modal( |modal| draw_frame( dc, modal ) );

	unsafe { EndPaint( overlay, &paint ) };
}

fn on_tick( overlay: HWND )
{
	unsafe { InvalidateRect( overlay, null(), 0 ) };

	let finished = with_modal( Modal::is_animation_over ).unwrap_or( false );

	if !finished
	{
		return;
	}

	unsafe { KillTimer( overlay, TIMER_ID ) };

	let closed = with_modal( |modal| modal.to == HIDDEN ).unwrap_or( false );

	if closed
	{
		finish_close();
	}
}

// A link opens its page, outside the card closes it, the rest of the card does nothing
fn on_click( point: POINT )
{
	if let Some( index ) = with_modal( |modal| modal.link_at( point ) ).flatten()
	{
		ui::shell_open( AUTHORS[ index ].url, None, None );

		return;
	}

	let outside = with_modal( |modal| !contains( &modal.current_card(), point ) ).unwrap_or( false );

	if outside
	{
		close();
	}
}

fn set_hovered_link( overlay: HWND, link: Option<usize> )
{
	let changed = with_modal_mut( |modal| modal.set_link( link ) ).unwrap_or( false );

	if changed
	{
		unsafe { InvalidateRect( overlay, null(), 0 ) };
	}
}

// WM_MOUSELEAVE has to be asked for again after each one
fn on_mouse_move( overlay: HWND, point: POINT )
{
	let mut track = TRACKMOUSEEVENT { cbSize: size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: overlay, dwHoverTime: 0 };

	unsafe { TrackMouseEvent( &mut track ) };

	let link = with_modal( |modal| modal.link_at( point ) ).flatten();

	set_hovered_link( overlay, link );
}

fn on_mouse_leave( overlay: HWND )
{
	set_hovered_link( overlay, None );
}

// A hand over a link; false leaves the class arrow. Comes before WM_MOUSEMOVE, so the position is read here
fn on_set_cursor( overlay: HWND ) -> bool
{
	let mut point = POINT { x: 0, y: 0 };

	unsafe
	{
		GetCursorPos( &mut point );
		ScreenToClient( overlay, &mut point );
	}

	let over_link = with_modal( |modal| modal.link_at( point ).is_some() ).unwrap_or( false );

	if over_link
	{
		unsafe { SetCursor( LoadCursorW( null_mut(), IDC_HAND ) ) };
	}

	over_link
}

fn on_key_down( key: u16 )
{
	if key == VK_ESCAPE
	{
		close();
	}
}

unsafe extern "system" fn overlay_proc( overlay: HWND, message: u32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT
{
	match message
	{
		WM_PAINT =>
		{
			on_paint( overlay );

			0
		}

		WM_ERASEBKGND => 1,

		WM_TIMER =>
		{
			on_tick( overlay );

			0
		}

		WM_LBUTTONDOWN =>
		{
			on_click( ui::lparam_point( lparam ) );

			0
		}

		WM_MOUSEMOVE =>
		{
			on_mouse_move( overlay, ui::lparam_point( lparam ) );

			0
		}

		WM_MOUSELEAVE =>
		{
			on_mouse_leave( overlay );

			0
		}

		WM_SETCURSOR if on_set_cursor( overlay ) => 1,

		WM_KEYDOWN =>
		{
			on_key_down( wparam as u16 );

			0
		}

		// Esc reaches the overlay through IsDialogMessage
		WM_GETDLGCODE => DLGC_WANTALLKEYS as LRESULT,

		_ => unsafe { DefWindowProcW( overlay, message, wparam, lparam ) },
	}
}
