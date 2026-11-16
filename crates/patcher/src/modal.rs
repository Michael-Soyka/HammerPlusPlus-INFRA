//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: A card sliding up from the bottom over a dimmed snapshot of the installer window,
//			shared by the authors card and the shortcut prompt
//
//=============================================================================//

use crate::gdi::{ self, Canvas };
use crate::ui::{ self, scale, wide };
use std::cell::RefCell;
use std::ptr::{ null, null_mut };
use std::rc::Rc;
use std::time::{ Duration, Instant };
use windows_sys::Win32::Foundation::{ HWND, LPARAM, LRESULT, POINT, RECT, WPARAM };
use windows_sys::Win32::Graphics::Gdi::{ BeginPaint, BitBlt, EndPaint, GetDC, HDC, InvalidateRect, PAINTSTRUCT, RDW_ALLCHILDREN, RDW_UPDATENOW, RedrawWindow, ReleaseDC, SRCCOPY, ScreenToClient };
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::WM_MOUSELEAVE;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{ GetFocus, SetFocus, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, VK_ESCAPE, VK_RETURN };
use windows_sys::Win32::UI::WindowsAndMessaging::{ CreateWindowExW, DLGC_WANTALLKEYS, DefWindowProcW, DestroyWindow, GetClientRect, GetCursorPos, HWND_TOP, IDC_ARROW, IDC_HAND, KillTimer, LoadCursorW, RegisterClassExW, SWP_NOMOVE, SWP_NOSIZE, SetCursor, SetTimer, SetWindowPos, WM_ERASEBKGND, WM_GETDLGCODE, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_PAINT, WM_SETCURSOR, WM_TIMER, WNDCLASSEXW, WS_CHILD, WS_VISIBLE };

const CLASS: &str = "InfraHppModal";

const DIM_ALPHA: f32 = 160.0;
const ANIMATION: Duration = Duration::from_millis( 240 );
const FRAME_MS:  u32 = 10;
const TIMER_ID:  usize = 1;

// 0 is hidden below the window, 1 is open
const HIDDEN: f32 = 0.0;
const OPEN:   f32 = 1.0;

// What the card shows; items are its clickable parts, by index
pub trait Card
{
	// At 96 DPI
	fn size( &self ) -> (i32, i32);

	fn draw( &self, dc: HDC, card: &RECT, hovered: Option<usize> );

	fn item_at( &self, card: &RECT, point: POINT ) -> Option<usize>;

	// True closes the card
	fn on_item( &self, index: usize ) -> bool;

	// Clicked by Enter
	fn default_item( &self ) -> Option<usize>
	{
		None
	}
}

struct Modal
{
	overlay:  HWND,
	snapshot: Canvas,
	width:    i32,
	height:   i32,

	// Shared, so its handlers run without the modal borrowed
	card: Rc<dyn Card>,

	// Gets the focus back on closing
	focus: HWND,

	from:  f32,
	to:    f32,
	start: Instant,

	// Index of the item under the cursor
	hovered: Option<usize>,
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

pub fn contains( rect: &RECT, point: POINT ) -> bool
{
	point.x >= rect.left && point.x < rect.right && point.y >= rect.top && point.y < rect.bottom
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

	fn card_rect( &self, value: f32 ) -> RECT
	{
		let ( card_width, card_height ) = self.card.size();

		let width    = scale( card_width );
		let height   = scale( card_height );
		let left     = ( self.width - width ) / 2;
		let open_top = ( self.height - height ) / 2;
		let top      = open_top + ( ( 1.0 - value ) * ( self.height - open_top ) as f32 ) as i32;

		let card = RECT { left, top, right: left + width, bottom: top + height };

		card
	}

	fn current_card( &self ) -> RECT
	{
		self.card_rect( self.value() )
	}

	fn item_at( &self, point: POINT ) -> Option<usize>
	{
		self.card.item_at( &self.current_card(), point )
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

	// True when the hovered item changed
	fn set_hovered( &mut self, hovered: Option<usize> ) -> bool
	{
		std::mem::replace( &mut self.hovered, hovered ) != hovered
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

pub fn open( card: Rc<dyn Card> )
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
		width,
		height,
		card,
		focus,
		from:    HIDDEN,
		to:      OPEN,
		start:   Instant::now(),
		hovered: None,
	};

	MODAL.with_borrow_mut( |slot| *slot = Some( modal ) );

	unsafe
	{
		SetWindowPos( overlay, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE );
		SetFocus( overlay );
		SetTimer( overlay, TIMER_ID, FRAME_MS, None );
	}
}

// Repaints an open card
pub fn invalidate()
{
	if let Some( overlay ) = with_modal( |modal| modal.overlay )
	{
		unsafe { InvalidateRect( overlay, null(), 0 ) };
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

// Composed off screen, so a frame never flickers
fn draw_frame( dc: HDC, modal: &Modal )
{
	let value  = modal.value();
	let width  = modal.width;
	let height = modal.height;
	let frame  = Canvas::new( dc, width, height );

	unsafe { BitBlt( frame.dc, 0, 0, width, height, modal.snapshot.dc, 0, 0, SRCCOPY ) };

	gdi::dim( frame.dc, width, height, ( value * DIM_ALPHA ) as u8 );
	modal.card.draw( frame.dc, &modal.card_rect( value ), modal.hovered );

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

fn on_item( index: usize )
{
	let Some( card ) = with_modal( |modal| modal.card.clone() ) else
	{
		return;
	};

	if card.on_item( index )
	{
		close();
	}
}

// An item runs its action, outside the card closes it, the rest of the card does nothing
fn on_click( point: POINT )
{
	if let Some( index ) = with_modal( |modal| modal.item_at( point ) ).flatten()
	{
		on_item( index );

		return;
	}

	let outside = with_modal( |modal| !contains( &modal.current_card(), point ) ).unwrap_or( false );

	if outside
	{
		close();
	}
}

fn set_hovered( overlay: HWND, hovered: Option<usize> )
{
	let changed = with_modal_mut( |modal| modal.set_hovered( hovered ) ).unwrap_or( false );

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

	let hovered = with_modal( |modal| modal.item_at( point ) ).flatten();

	set_hovered( overlay, hovered );
}

fn on_mouse_leave( overlay: HWND )
{
	set_hovered( overlay, None );
}

// A hand over an item; false leaves the class arrow. Comes before WM_MOUSEMOVE, so the position is read here
fn on_set_cursor( overlay: HWND ) -> bool
{
	let mut point = POINT { x: 0, y: 0 };

	unsafe
	{
		GetCursorPos( &mut point );
		ScreenToClient( overlay, &mut point );
	}

	let over_item = with_modal( |modal| modal.item_at( point ).is_some() ).unwrap_or( false );

	if over_item
	{
		unsafe { SetCursor( LoadCursorW( null_mut(), IDC_HAND ) ) };
	}

	over_item
}

fn on_key_down( key: u16 )
{
	if key == VK_ESCAPE
	{
		close();

		return;
	}

	if key != VK_RETURN
	{
		return;
	}

	if let Some( index ) = with_modal( |modal| modal.card.default_item() ).flatten()
	{
		on_item( index );
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

		// Esc and Enter reach the overlay through IsDialogMessage
		WM_GETDLGCODE => DLGC_WANTALLKEYS as LRESULT,

		_ => unsafe { DefWindowProcW( overlay, message, wparam, lparam ) },
	}
}
