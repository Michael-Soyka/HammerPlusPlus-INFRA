//=========== Copyright 2026 Michael Soyka, All rights reserved. ==============//
//
// Purpose: Installer window: asks for the INFRA and Hammer++ build folders,
//			installs, removes and launches
//
//=============================================================================//

use crate::gdi::{ self, Canvas };
use crate::palette::{ ACCENT, ACCENT_HOVER, ACCENT_PRESSED, BACKGROUND, BAD, BORDER, BUTTON, BUTTON_TEXT, DIM, FIELD, GOOD, HOVER, PRESSED, TEXT, WARNING };
use crate::{ authors, avatar, background, gdiplus, install, steam };
use std::cell::{ Cell, RefCell };
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::{ Path, PathBuf };
use std::ptr::{ null, null_mut };
use windows_sys::Win32::Foundation::{ COLORREF, HWND, LPARAM, LRESULT, MAX_PATH, POINT, RECT, WPARAM };
use windows_sys::Win32::Graphics::Dwm::{ DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute };
use windows_sys::Win32::Graphics::Gdi::{ BeginPaint, CreateSolidBrush, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_PATH_ELLIPSIS, DT_SINGLELINE, DT_TOP, DT_VCENTER, DT_WORDBREAK, EndPaint, FW_NORMAL, FW_SEMIBOLD, GetDC, GetStockObject, HBRUSH, HDC, HFONT, InflateRect, InvalidateRect, MapWindowPoints, NULL_BRUSH, PAINTSTRUCT, ReleaseDC, SetBkColor, SetTextColor };
use windows_sys::Win32::System::Com::{ COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoTaskMemFree, CoUninitialize };
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{ DRAWITEMSTRUCT, EM_SETCUEBANNER, ODS_DISABLED, ODS_FOCUS, ODS_SELECTED, WM_MOUSELEAVE };
use windows_sys::Win32::UI::HiDpi::{ AdjustWindowRectExForDpi, DPI_AWARENESS_CONTEXT_SYSTEM_AWARE, GetDpiForSystem, GetSystemMetricsForDpi, SetProcessDpiAwarenessContext };
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{ EnableWindow, SetFocus, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent };
use windows_sys::Win32::UI::Shell::{ BFFM_INITIALIZED, BFFM_SETSELECTIONW, BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW, DefSubclassProc, DragAcceptFiles, DragFinish, DragQueryFileW, DragQueryPoint, HDROP, SHBrowseForFolderW, SHGetPathFromIDListW, SetWindowSubclass, ShellExecuteW };
use windows_sys::Win32::UI::WindowsAndMessaging::{ BN_CLICKED, BS_OWNERDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DispatchMessageW, EN_CHANGE, ES_AUTOHSCROLL, GetClientRect, GetDlgCtrlID, GetDlgItem, GetMessageW, GetWindowTextLengthW, GetWindowTextW, HICON, IDC_ARROW, IDC_WAIT, IMAGE_ICON, IsDialogMessageW, LR_DEFAULTCOLOR, LoadCursorW, LoadImageW, MSG, PostQuitMessage, RegisterClassExW, SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON, SW_HIDE, SW_SHOW, SW_SHOWDEFAULT, SW_SHOWNORMAL, SendMessageW, SetCursor, SetTimer, SetWindowTextW, ShowWindow, TranslateMessage, WM_COMMAND, WM_CREATE, WM_CTLCOLORBTN, WM_CTLCOLOREDIT, WM_DESTROY, WM_DRAWITEM, WM_DROPFILES, WM_ERASEBKGND, WM_MOUSEMOVE, WM_PAINT, WM_SETFONT, WM_TIMER, WNDCLASSEXW, WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE };

const DOWNLOAD_URL: &str = "https://ficool2.github.io/HammerPlusPlus-Website/download.html";
const CLASS:        &str = "InfraHppSetup";

// Client area at 96 DPI
const WIDTH:       i32 = 600;
const HEIGHT:      i32 = 382;
const DEFAULT_DPI: i32 = 96;

// ShellExecute reports success with a value above 32
const SHELL_EXECUTE_OK: isize = 32;

// MAKEINTRESOURCE( 1 ): the icon of app.rc
const APP_ICON: *const u16 = std::ptr::without_provenance( 1 );

const ID_TITLE:        i32 = 100;
const ID_GAME_LABEL:   i32 = 101;
const ID_GAME:         i32 = 102;
const ID_GAME_BROWSE:  i32 = 103;
const ID_GAME_STATUS:  i32 = 104;
const ID_HPP_LABEL:    i32 = 105;
const ID_HPP:          i32 = 106;
const ID_HPP_DOWNLOAD: i32 = 107;
const ID_HPP_BROWSE:   i32 = 108;
const ID_HPP_STATUS:   i32 = 109;
const ID_VERSION:      i32 = 110;
const ID_RESULT:       i32 = 111;
const ID_INSTALL:      i32 = 112;
const ID_REMOVE:       i32 = 113;
const ID_REINSTALL:    i32 = 114;
const ID_LAUNCH:       i32 = 115;
const ID_INFO:         i32 = 116;

// At 96 DPI
#[ derive( Clone, Copy ) ]
struct Bounds
{
	x:      i32,
	y:      i32,
	width:  i32,
	height: i32,
}

// Path edits sit inside drawn rounded frames with a padding
const GAME_FRAME:    Bounds = Bounds { x: 16, y: 84, width: 470, height: 32 };
const HPP_FRAME:     Bounds = Bounds { x: 16, y: 176, width: 372, height: 32 };
const FIELD_PADDING: i32 = 7;
const FIELD_RADIUS:  i32 = 6;
const FOCUS_RADIUS:  i32 = 5;
const FOCUS_INSET:   i32 = 2;
const INFO_RADIUS:   i32 = 9;

// Files dropped below this line go to the Hammer++ field
const HPP_SECTION_TOP: i32 = 150;

const BACKGROUND_TIMER: usize = 1;

const LINE:      u32 = DT_LEFT | DT_TOP | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS;
const PATH_LINE: u32 = DT_LEFT | DT_TOP | DT_SINGLELINE | DT_NOPREFIX | DT_PATH_ELLIPSIS;
const PARAGRAPH: u32 = DT_LEFT | DT_TOP | DT_WORDBREAK | DT_NOPREFIX;

// Text drawn over the video: no controls, which would cover it with their background
struct Label
{
	id:     i32,
	bounds: Bounds,
	title:  bool,
	format: u32,
}

const LABELS: [Label; 7] =
[
	Label { id: ID_TITLE, bounds: Bounds { x: 16, y: 14, width: 568, height: 36 }, title: true, format: LINE },
	Label { id: ID_GAME_LABEL, bounds: Bounds { x: 16, y: 62, width: 568, height: 20 }, title: false, format: LINE },
	Label { id: ID_GAME_STATUS, bounds: Bounds { x: 16, y: 122, width: 568, height: 20 }, title: false, format: PATH_LINE },
	Label { id: ID_HPP_LABEL, bounds: Bounds { x: 16, y: 154, width: 568, height: 20 }, title: false, format: LINE },
	Label { id: ID_HPP_STATUS, bounds: Bounds { x: 16, y: 214, width: 568, height: 20 }, title: false, format: PATH_LINE },
	Label { id: ID_VERSION, bounds: Bounds { x: 16, y: 246, width: 568, height: 20 }, title: false, format: LINE },
	Label { id: ID_RESULT, bounds: Bounds { x: 16, y: 274, width: 568, height: 48 }, title: false, format: PARAGRAPH },
];

const STATIC_LABELS: [(i32, &str, COLORREF); 3] =
[
	( ID_TITLE, "Hammer++ patcher for INFRA", TEXT ),
	( ID_GAME_LABEL, "INFRA folder", DIM ),
	( ID_HPP_LABEL, "Hammer++ build for Portal 2", DIM ),
];

struct Control
{
	class:  &'static str,
	text:   &'static str,
	style:  u32,
	id:     i32,
	bounds: Bounds,
}

const EDIT_STYLE:   u32 = WS_TABSTOP | ES_AUTOHSCROLL as u32;
const BUTTON_STYLE: u32 = WS_TABSTOP | BS_OWNERDRAW as u32;

const fn field_edit( frame: Bounds ) -> Bounds
{
	Bounds { x: frame.x + FIELD_PADDING, y: frame.y + FIELD_PADDING, width: frame.width - ( 2 * FIELD_PADDING ), height: frame.height - ( 2 * FIELD_PADDING ) }
}

const fn button( text: &'static str, id: i32, x: i32, y: i32, width: i32, height: i32 ) -> Control
{
	Control { class: "BUTTON", text, style: BUTTON_STYLE, id, bounds: Bounds { x, y, width, height } }
}

const fn edit( id: i32, frame: Bounds ) -> Control
{
	Control { class: "EDIT", text: "", style: EDIT_STYLE, id, bounds: field_edit( frame ) }
}

const CONTROLS: [Control; 10] =
[
	edit( ID_GAME, GAME_FRAME ),
	button( "Browse\u{2026}", ID_GAME_BROWSE, 494, 84, 90, 32 ),
	edit( ID_HPP, HPP_FRAME ),
	button( "Download", ID_HPP_DOWNLOAD, 396, 176, 90, 32 ),
	button( "Browse\u{2026}", ID_HPP_BROWSE, 494, 176, 90, 32 ),
	button( "Install", ID_INSTALL, 444, 334, 140, 34 ),
	button( "Remove", ID_REMOVE, 232, 334, 112, 34 ),
	button( "Reinstall", ID_REINSTALL, 352, 334, 112, 34 ),
	button( "Launch", ID_LAUNCH, 472, 334, 112, 34 ),
	button( "i", ID_INFO, 16, 334, 34, 34 ),
];

// Repainted on every frame: their rounded corners show the video
const BUTTONS: [i32; 8] = [ ID_GAME_BROWSE, ID_HPP_DOWNLOAD, ID_HPP_BROWSE, ID_INSTALL, ID_REMOVE, ID_REINSTALL, ID_LAUNCH, ID_INFO ];

// Install and Launch share a place with this row
const INSTALLED_ROW: [i32; 3] = [ ID_REMOVE, ID_REINSTALL, ID_LAUNCH ];

const CUE_BANNERS: [(i32, &str); 2] =
[
	( ID_GAME, "C:\\Program Files (x86)\\Steam\\steamapps\\common\\infra" ),
	( ID_HPP, "hammerplusplus_portal2_buildNNNN" ),
];

struct ButtonColors
{
	normal:  COLORREF,
	hover:   COLORREF,
	pressed: COLORREF,
}

const PLAIN_COLORS:  ButtonColors = ButtonColors { normal: BUTTON, hover: HOVER, pressed: PRESSED };
const ACCENT_COLORS: ButtonColors = ButtonColors { normal: ACCENT, hover: ACCENT_HOVER, pressed: ACCENT_PRESSED };

struct StatusLine
{
	text:  String,
	color: COLORREF,
}

#[ derive( Clone, Default ) ]
struct State
{
	game:      Option<PathBuf>,
	installed: bool,
	version:   Option<String>,

	// The bin of a checked Portal 2 build
	hpp: Option<PathBuf>,
}

struct App
{
	window:     Cell<HWND>,
	font:       Cell<HFONT>,
	title_font: Cell<HFONT>,
	field:      Cell<HBRUSH>,
	dpi:        Cell<i32>,

	// Id of the button under the cursor, 0 for none
	hovered: Cell<i32>,

	// The dimmed video frame under everything
	scene: RefCell<Option<Canvas>>,

	labels: RefCell<HashMap<i32, (String, COLORREF)>>,
	state:  RefCell<State>,
}

thread_local!
{
	static APP: App = App
	{
		window:     Cell::new( null_mut() ),
		font:       Cell::new( null_mut() ),
		title_font: Cell::new( null_mut() ),
		field:      Cell::new( null_mut() ),
		dpi:        Cell::new( DEFAULT_DPI ),
		hovered:    Cell::new( 0 ),
		scene:      RefCell::new( None ),
		labels:     RefCell::new( HashMap::new() ),
		state:      RefCell::new( State::default() ),
	};
}

fn app<R>( read: impl FnOnce(&App) -> R ) -> R
{
	APP.with( read )
}

// A copy: Win32 calls send messages back, a held borrow would panic
fn state() -> State
{
	app( |app| app.state.borrow().clone() )
}

fn update_state( update: impl FnOnce(&mut State) )
{
	app( |app| update( &mut app.state.borrow_mut() ) );
}

//-----------------------------------------------------------------------------
// Helpers
//-----------------------------------------------------------------------------

pub fn wide( text: impl AsRef<OsStr> ) -> Vec<u16>
{
	text.as_ref().encode_wide().chain( std::iter::once( 0 ) ).collect()
}

fn from_wide( buffer: &[u16] ) -> String
{
	let len = buffer.iter().position( |&c| c == 0 ).unwrap_or( buffer.len() );

	String::from_utf16_lossy( &buffer[ ..len ] )
}

fn wide_or_null( text: &Option<Vec<u16>> ) -> *const u16
{
	let Some( text ) = text else
	{
		return null();
	};

	text.as_ptr()
}

pub fn scale( value: i32 ) -> i32
{
	( value * app( |app| app.dpi.get() ) ) / DEFAULT_DPI
}

fn scale_bounds( bounds: Bounds ) -> RECT
{
	RECT { left: scale( bounds.x ), top: scale( bounds.y ), right: scale( bounds.x + bounds.width ), bottom: scale( bounds.y + bounds.height ) }
}

// Client coordinates of a mouse message
pub fn lparam_point( lparam: LPARAM ) -> POINT
{
	let x = ( lparam & 0xffff ) as i16 as i32;
	let y = ( ( lparam >> 16 ) & 0xffff ) as i16 as i32;

	POINT { x, y }
}

pub fn window() -> HWND
{
	app( |app| app.window.get() )
}

fn item( id: i32 ) -> HWND
{
	unsafe { GetDlgItem( window(), id ) }
}

fn set_text( control: HWND, text: &str )
{
	let text = wide( text );

	unsafe { SetWindowTextW( control, text.as_ptr() ) };
}

fn control_text( control: HWND ) -> String
{
	let length = unsafe { GetWindowTextLengthW( control ) };

	let mut buffer = vec![ 0u16; length as usize + 1 ];

	unsafe { GetWindowTextW( control, buffer.as_mut_ptr(), buffer.len() as i32 ) };

	from_wide( &buffer )
}

// Field text without the quotes a copied path often has
fn field_text( id: i32 ) -> String
{
	let text = control_text( item( id ) ).trim().trim_matches( '"' ).to_string();

	text
}

// No erasing: everything paints its whole area, erasing would only flicker
fn invalidate( control: HWND )
{
	unsafe { InvalidateRect( control, null(), 0 ) };
}

fn client_size( window: HWND ) -> (i32, i32)
{
	let mut client = RECT::default();

	unsafe { GetClientRect( window, &mut client ) };

	( client.right, client.bottom )
}

pub fn shell_open( file: impl AsRef<OsStr>, args: Option<&str>, dir: Option<&Path> ) -> bool
{
	let verb = wide( "open" );
	let file = wide( file );
	let args = args.map( wide );
	let dir  = dir.map( wide );

	let result = unsafe { ShellExecuteW( window(), verb.as_ptr(), file.as_ptr(), wide_or_null( &args ), wide_or_null( &dir ), SW_SHOWNORMAL ) };

	result as isize > SHELL_EXECUTE_OK
}

//-----------------------------------------------------------------------------
// State
//-----------------------------------------------------------------------------

fn is_outdated( state: &State ) -> bool
{
	state.installed && state.version.as_deref().is_none_or( |version| install::compare_versions( version, install::VERSION ).is_lt() )
}

fn describe_version( state: &State ) -> String
{
	let installer = install::VERSION;

	if !state.installed
	{
		return format!( "Installer version {installer}" );
	}

	let Some( version ) = &state.version else
	{
		return format!( "Installed: unknown, older than versioning  \u{b7}  Installer: {installer}  \u{2014}  outdated" );
	};

	let verdict = match install::compare_versions( version, installer )
	{
		Ordering::Less => "outdated",
		Ordering::Equal => "up to date",
		Ordering::Greater => "newer than this installer",
	};

	let text = format!( "Installed: {version}  \u{b7}  Installer: {installer}  \u{2014}  {verdict}" );

	text
}

fn describe_game( input: &str, game: Option<&Path>, installed: bool ) -> StatusLine
{
	if input.is_empty()
	{
		return StatusLine { text: "Enter, browse or drop the INFRA folder".to_string(), color: DIM };
	}

	let Some( dir ) = game else
	{
		return StatusLine { text: "No bin\\hammer.exe here: not an INFRA folder".to_string(), color: BAD };
	};

	if installed
	{
		return StatusLine { text: format!( "Installed: {}", dir.display() ), color: WARNING };
	}

	StatusLine { text: format!( "INFRA found: {}", dir.display() ), color: GOOD }
}

fn describe_hpp( input: &str, check: Option<&Result<(), String>> ) -> StatusLine
{
	if input.is_empty()
	{
		return StatusLine { text: "Download the Portal 2 build, unpack it and pick its folder".to_string(), color: DIM };
	}

	let status = match check
	{
		None => StatusLine { text: "hammerplusplus.exe is not found here or in its bin".to_string(), color: BAD },
		Some( Err( error ) ) => StatusLine { text: error.clone(), color: BAD },
		Some( Ok( () ) ) => StatusLine { text: format!( "Hammer++ build: {input}" ), color: GOOD },
	};

	status
}

fn show_buttons( ids: &[i32], show: i32 )
{
	for &id in ids
	{
		unsafe { ShowWindow( item( id ), show ) };
	}
}

// Not installed: Install alone. Installed: Remove, Reinstall (Update when outdated) and Launch
fn update_buttons( state: &State )
{
	let mut reinstall = "Reinstall";

	if is_outdated( state )
	{
		reinstall = "Update";
	}

	set_text( item( ID_REINSTALL ), reinstall );

	unsafe
	{
		EnableWindow( item( ID_INSTALL ), ( state.game.is_some() && state.hpp.is_some() ).into() );
		EnableWindow( item( ID_REINSTALL ), state.hpp.is_some().into() );
	}

	// Install and Launch share a place: hide first, then show
	if state.installed
	{
		show_buttons( &[ ID_INSTALL ], SW_HIDE );
		show_buttons( &INSTALLED_ROW, SW_SHOW );
	}
	else
	{
		show_buttons( &INSTALLED_ROW, SW_HIDE );
		show_buttons( &[ ID_INSTALL ], SW_SHOW );
	}

	invalidate( item( ID_INSTALL ) );
	invalidate( item( ID_REINSTALL ) );
}

fn set_label( id: i32, text: &str, color: COLORREF )
{
	app( |app| app.labels.borrow_mut().insert( id, ( text.to_string(), color ) ) );

	let Some( label ) = LABELS.iter().find( |label| label.id == id ) else
	{
		return;
	};

	unsafe { InvalidateRect( window(), &scale_bounds( label.bounds ), 0 ) };
}

fn set_status( id: i32, status: &StatusLine )
{
	set_label( id, &status.text, status.color );
}

fn refresh_game()
{
	let input     = field_text( ID_GAME );
	let game      = install::game_dir( Path::new( &input ) );
	let installed = game.as_deref().is_some_and( install::is_installed );
	let version   = game.as_deref().filter( |_| installed ).and_then( install::installed_version );

	set_status( ID_GAME_STATUS, &describe_game( &input, game.as_deref(), installed ) );

	update_state(
		|state|
		{
			state.game      = game;
			state.installed = installed;
			state.version   = version;
		}
	);

	let state = state();

	set_label( ID_VERSION, &describe_version( &state ), DIM );
	update_buttons( &state );
}

fn refresh_hpp()
{
	let input = field_text( ID_HPP );
	let bin   = install::hpp_bin( Path::new( &input ) );
	let check = bin.as_deref().map( install::validate_build );

	set_status( ID_HPP_STATUS, &describe_hpp( &input, check.as_ref() ) );

	let valid = check.is_some_and( |check| check.is_ok() );
	let hpp   = bin.filter( |_| valid );

	update_state( |state| state.hpp = hpp );
	update_buttons( &state() );
}

fn set_result( text: &str, color: COLORREF )
{
	set_label( ID_RESULT, text, color );
}

fn show_result( result: Result<String, String> )
{
	refresh_game();

	match result
	{
		Ok( text ) => set_result( &text, GOOD ),
		Err( text ) => set_result( &text, BAD ),
	}
}

//-----------------------------------------------------------------------------
// Events
//-----------------------------------------------------------------------------

fn on_game_change()
{
	set_result( "", TEXT );
	refresh_game();
}

fn on_hpp_change()
{
	set_result( "", TEXT );
	refresh_hpp();
}

// Install and Reinstall: install() writes over a previous installation
fn on_install()
{
	let state = state();

	let ( Some( game ), Some( hpp ) ) = ( state.game, state.hpp ) else
	{
		return;
	};

	unsafe { SetCursor( LoadCursorW( null_mut(), IDC_WAIT ) ) };

	show_result( install::install( &game, &hpp ) );
}

fn installed_game() -> Option<PathBuf>
{
	let state = state();

	state.game.filter( |_| state.installed )
}

fn on_remove()
{
	let Some( game ) = installed_game() else
	{
		return;
	};

	show_result( install::uninstall( &game ) );
}

fn on_launch()
{
	let Some( game ) = installed_game() else
	{
		return;
	};

	let started = shell_open( install::launcher_exe( &game ), Some( &install::launch_args( &game ) ), Some( &game.join( "bin" ) ) );

	if started
	{
		set_result( "Hammer++ started", GOOD );

		return;
	}

	set_result( "Failed to start hammerplusplus.exe", BAD );
}

fn on_info()
{
	authors::open();
}

fn on_download()
{
	if !shell_open( DOWNLOAD_URL, None, None )
	{
		set_result( &format!( "Failed to open {DOWNLOAD_URL}" ), BAD );
	}
}

// Selects the current field path once the dialog opens
unsafe extern "system" fn browse_callback( window: HWND, message: u32, _: LPARAM, data: LPARAM ) -> i32
{
	if message == BFFM_INITIALIZED
	{
		unsafe { SendMessageW( window, BFFM_SETSELECTIONW, 1, data ) };
	}

	0
}

fn on_browse( field: i32, title: &str )
{
	let current = wide( field_text( field ) );
	let title   = wide( title );

	let mut display = [ 0u16; MAX_PATH as usize ];

	let info = BROWSEINFOW
	{
		hwndOwner:      window(),
		pidlRoot:       null_mut(),
		pszDisplayName: display.as_mut_ptr(),
		lpszTitle:      title.as_ptr(),
		ulFlags:        BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE,
		lpfn:           Some( browse_callback ),
		lParam:         current.as_ptr() as LPARAM,
		iImage:         0,
	};

	let list = unsafe { SHBrowseForFolderW( &info ) };

	if list.is_null()
	{
		return;
	}

	let mut path = [ 0u16; MAX_PATH as usize ];

	let found = unsafe { SHGetPathFromIDListW( list, path.as_mut_ptr() ) } != 0;

	unsafe { CoTaskMemFree( list.cast() ) };

	if found
	{
		set_text( item( field ), &from_wide( &path ) );
	}
}

// A dropped file stands for its folder; the drop point picks the field
fn on_drop_files( drop: HDROP )
{
	let mut buffer = [ 0u16; MAX_PATH as usize ];
	let mut point  = POINT { x: 0, y: 0 };

	let length = unsafe { DragQueryFileW( drop, 0, buffer.as_mut_ptr(), MAX_PATH ) };

	unsafe
	{
		DragQueryPoint( drop, &mut point );
		DragFinish( drop );
	}

	if length == 0
	{
		return;
	}

	let mut path = PathBuf::from( from_wide( &buffer ) );

	if !path.is_dir()
	{
		path.pop();
	}

	let mut field = ID_GAME;

	if point.y >= scale( HPP_SECTION_TOP )
	{
		field = ID_HPP;
	}

	set_text( item( field ), &path.display().to_string() );
}

fn on_command( id: i32, code: u32 )
{
	match ( id, code )
	{
		( ID_GAME, EN_CHANGE ) => on_game_change(),
		( ID_HPP, EN_CHANGE ) => on_hpp_change(),
		( ID_GAME_BROWSE, BN_CLICKED ) => on_browse( ID_GAME, "INFRA folder" ),
		( ID_HPP_BROWSE, BN_CLICKED ) => on_browse( ID_HPP, "Hammer++ build for Portal 2" ),
		( ID_HPP_DOWNLOAD, BN_CLICKED ) => on_download(),
		( ID_INSTALL | ID_REINSTALL, BN_CLICKED ) => on_install(),
		( ID_REMOVE, BN_CLICKED ) => on_remove(),
		( ID_LAUNCH, BN_CLICKED ) => on_launch(),
		( ID_INFO, BN_CLICKED ) => on_info(),
		_ => {}
	}
}

//-----------------------------------------------------------------------------
// Drawing
//-----------------------------------------------------------------------------

fn copy_scene_of( app: &App, dc: HDC, target: &RECT, window_x: i32, window_y: i32 )
{
	let scene = app.scene.borrow();

	let Some( scene ) = scene.as_ref() else
	{
		return;
	};

	scene.copy_to( dc, target, window_x, window_y );
}

// Copies the scene under the window point into the DC at the target
fn copy_scene( dc: HDC, target: &RECT, window_x: i32, window_y: i32 )
{
	app( |app| copy_scene_of( app, dc, target, window_x, window_y ) );
}

fn is_disabled( item: &DRAWITEMSTRUCT ) -> bool
{
	( item.itemState & ODS_DISABLED ) != 0
}

fn pick_fill( colors: &ButtonColors, selected: bool, hovered: bool ) -> COLORREF
{
	if selected
	{
		return colors.pressed;
	}

	if hovered
	{
		return colors.hover;
	}

	colors.normal
}

fn button_fill( item: &DRAWITEMSTRUCT ) -> COLORREF
{
	if is_disabled( item )
	{
		return BUTTON;
	}

	let id       = item.CtlID as i32;
	let selected = ( item.itemState & ODS_SELECTED ) != 0;
	let hovered  = app( |app| app.hovered.get() ) == id;

	let mut colors = &PLAIN_COLORS;

	if id == ID_INSTALL || id == ID_LAUNCH
	{
		colors = &ACCENT_COLORS;
	}

	pick_fill( colors, selected, hovered )
}

fn draw_button_text( item: &DRAWITEMSTRUCT )
{
	let mut color = BUTTON_TEXT;

	if is_disabled( item )
	{
		color = DIM;
	}

	let text = control_text( item.hwndItem );
	let font = app( |app| app.font.get() );

	gdi::draw_text( item.hDC, &text, item.rcItem, font, color, DT_CENTER | DT_VCENTER | DT_SINGLELINE );
}

// DrawFocusRect inverts to orange over the accent, an outline stays readable
fn draw_button_focus( item: &DRAWITEMSTRUCT )
{
	let mut focus = item.rcItem;

	unsafe { InflateRect( &mut focus, -scale( FOCUS_INSET ), -scale( FOCUS_INSET ) ) };

	gdi::outline_round_rect( item.hDC, &focus, TEXT, FOCUS_RADIUS );
}

// The circle around the "i" of the info button
fn draw_info_circle( item: &DRAWITEMSTRUCT )
{
	let rect     = item.rcItem;
	let center_x = ( rect.left + rect.right ) / 2;
	let center_y = ( rect.top + rect.bottom ) / 2;
	let radius   = scale( INFO_RADIUS );
	let circle   = RECT { left: center_x - radius, top: center_y - radius, right: center_x + radius, bottom: center_y + radius };

	gdi::outline_round_rect( item.hDC, &circle, BUTTON_TEXT, 2 * INFO_RADIUS );
}

fn draw_button( item: &DRAWITEMSTRUCT, window_x: i32, window_y: i32 )
{
	let fill = button_fill( item );

	let mut edge = fill;

	if is_disabled( item )
	{
		edge = BORDER;
	}

	copy_scene( item.hDC, &item.rcItem, window_x, window_y );

	gdi::fill_round_rect( item.hDC, &item.rcItem, fill, edge, FIELD_RADIUS );
	draw_button_text( item );

	if item.CtlID as i32 == ID_INFO
	{
		draw_info_circle( item );
	}

	if ( item.itemState & ODS_FOCUS ) != 0
	{
		draw_button_focus( item );
	}
}

// Composed off screen over the scene, so a frame never flickers
fn on_draw_button( item: &DRAWITEMSTRUCT )
{
	let rect   = item.rcItem;
	let width  = rect.right - rect.left;
	let height = rect.bottom - rect.top;

	let mut origin = POINT { x: rect.left, y: rect.top };

	unsafe { MapWindowPoints( item.hwndItem, window(), &mut origin, 1 ) };

	let canvas = Canvas::new( item.hDC, width, height );
	let local  = DRAWITEMSTRUCT { hDC: canvas.dc, rcItem: RECT { left: 0, top: 0, right: width, bottom: height }, ..*item };

	draw_button( &local, origin.x, origin.y );

	canvas.copy_to( item.hDC, &rect, 0, 0 );
}

fn label_font( label: &Label ) -> HFONT
{
	if label.title
	{
		return app( |app| app.title_font.get() );
	}

	app( |app| app.font.get() )
}

fn draw_labels( dc: HDC )
{
	let labels = app( |app| app.labels.borrow().clone() );

	for label in &LABELS
	{
		if let Some( ( text, color ) ) = labels.get( &label.id )
		{
			gdi::draw_text( dc, text, scale_bounds( label.bounds ), label_font( label ), *color, label.format );
		}
	}
}

fn on_paint()
{
	let window = window();

	let mut paint = PAINTSTRUCT::default();
	let dc        = unsafe { BeginPaint( window, &mut paint ) };

	let ( width, height ) = client_size( window );

	let frame = Canvas::new( dc, width, height );
	let whole = RECT { left: 0, top: 0, right: width, bottom: height };

	copy_scene( frame.dc, &whole, 0, 0 );

	gdi::fill_round_rect( frame.dc, &scale_bounds( GAME_FRAME ), FIELD, BORDER, FIELD_RADIUS );
	gdi::fill_round_rect( frame.dc, &scale_bounds( HPP_FRAME ), FIELD, BORDER, FIELD_RADIUS );
	draw_labels( frame.dc );

	frame.copy_to( dc, &whole, 0, 0 );

	unsafe { EndPaint( window, &paint ) };
}

fn create_scene( window: HWND, width: i32, height: i32 ) -> Canvas
{
	let window_dc = unsafe { GetDC( window ) };
	let canvas    = Canvas::new( window_dc, width, height );

	unsafe { ReleaseDC( window, window_dc ) };

	canvas
}

fn render_scene()
{
	let window = window();
	let ( width, height ) = client_size( window );

	app(
		|app|
		{
			let mut scene = app.scene.borrow_mut();
			let scene     = scene.get_or_insert_with( || create_scene( window, width, height ) );

			background::draw( scene.dc, width, height, BACKGROUND );
		}
	);
}

// Paused under the authors card: it shows a snapshot of the window anyway
fn on_background_tick()
{
	if authors::is_open() || !background::advance()
	{
		return;
	}

	render_scene();
	invalidate( window() );

	for id in BUTTONS
	{
		invalidate( item( id ) );
	}
}

fn on_ctl_color( dc: HDC, text: COLORREF, background: COLORREF, brush: HBRUSH ) -> LRESULT
{
	unsafe
	{
		SetTextColor( dc, text );
		SetBkColor( dc, background );
	}

	brush as LRESULT
}

//-----------------------------------------------------------------------------
// Buttons
//-----------------------------------------------------------------------------

// Windows asks for WM_MOUSELEAVE once per entry, so tracking starts on the first move over the button
fn on_button_hover( button: HWND )
{
	let id = unsafe { GetDlgCtrlID( button ) };

	if app( |app| app.hovered.replace( id ) ) == id
	{
		return;
	}

	let mut track = TRACKMOUSEEVENT { cbSize: size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: button, dwHoverTime: 0 };

	unsafe { TrackMouseEvent( &mut track ) };

	invalidate( button );
}

fn on_button_leave( button: HWND )
{
	let id = unsafe { GetDlgCtrlID( button ) };

	if app( |app| app.hovered.get() ) != id
	{
		return;
	}

	app( |app| app.hovered.set( 0 ) );

	invalidate( button );
}

// Owner-drawn buttons get no hover state from Windows: the mouse is watched here
unsafe extern "system" fn button_proc( button: HWND, message: u32, wparam: WPARAM, lparam: LPARAM, _: usize, _: usize ) -> LRESULT
{
	match message
	{
		WM_MOUSEMOVE => on_button_hover( button ),
		WM_MOUSELEAVE => on_button_leave( button ),
		_ => {}
	}

	unsafe { DefSubclassProc( button, message, wparam, lparam ) }
}

//-----------------------------------------------------------------------------
// Window
//-----------------------------------------------------------------------------

fn add_control( control: &Control )
{
	let class  = wide( control.class );
	let text   = wide( control.text );
	let bounds = control.bounds;
	let style  = WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | control.style;

	let handle = unsafe { CreateWindowExW( 0, class.as_ptr(), text.as_ptr(), style, scale( bounds.x ), scale( bounds.y ), scale( bounds.width ), scale( bounds.height ), window(), control.id as isize as _, null_mut(), null() ) };

	unsafe { SendMessageW( handle, WM_SETFONT, app( |app| app.font.get() ) as WPARAM, 0 ) };
}

fn add_controls()
{
	for ( id, text, color ) in STATIC_LABELS
	{
		set_label( id, text, color );
	}

	for control in &CONTROLS
	{
		add_control( control );
	}

	for id in BUTTONS
	{
		unsafe { SetWindowSubclass( item( id ), Some( button_proc ), 0, 0 ) };
	}

	for ( id, cue ) in CUE_BANNERS
	{
		let cue = wide( cue );

		unsafe { SendMessageW( item( id ), EM_SETCUEBANNER, 1, cue.as_ptr() as LPARAM ) };
	}
}

// Found folders fill the fields, which refreshes them through EN_CHANGE
fn fill_fields()
{
	if let Some( game ) = steam::find_infra()
	{
		set_text( item( ID_GAME ), &game.display().to_string() );
	}
	else
	{
		refresh_game();
	}

	if let Some( build ) = install::find_hpp_build()
	{
		set_text( item( ID_HPP ), &build.display().to_string() );
	}
	else
	{
		refresh_hpp();
	}
}

fn set_dark_title_bar( window: HWND )
{
	let dark: i32 = 1;

	unsafe { DwmSetWindowAttribute( window, DWMWA_USE_IMMERSIVE_DARK_MODE as u32, ( &raw const dark ).cast(), size_of::<i32>() as u32 ) };
}

fn on_create( window: HWND )
{
	app( |app| app.window.set( window ) );

	set_dark_title_bar( window );

	let font       = gdi::create_font( 15, FW_NORMAL as i32 );
	let title_font = gdi::create_font( 26, FW_SEMIBOLD as i32 );

	app( |app| app.font.set( font ) );
	app( |app| app.title_font.set( title_font ) );

	render_scene();
	add_controls();

	unsafe
	{
		DragAcceptFiles( window, 1 );
		SetTimer( window, BACKGROUND_TIMER, background::TICK_MS, None );
	}

	fill_fields();
	authors::load_avatars( window );

	unsafe { SetFocus( item( ID_GAME ) ) };
}

fn loword( wparam: WPARAM ) -> u32
{
	( wparam & 0xffff ) as u32
}

fn hiword( wparam: WPARAM ) -> u32
{
	( ( wparam >> 16 ) & 0xffff ) as u32
}

unsafe extern "system" fn window_proc( window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT
{
	match message
	{
		WM_CREATE =>
		{
			on_create( window );

			0
		}

		WM_COMMAND =>
		{
			on_command( loword( wparam ) as i32, hiword( wparam ) );

			0
		}

		WM_DROPFILES =>
		{
			on_drop_files( wparam as HDROP );

			0
		}

		WM_DRAWITEM =>
		{
			on_draw_button( unsafe { &*( lparam as *const DRAWITEMSTRUCT ) } );

			1
		}

		WM_PAINT =>
		{
			on_paint();

			0
		}

		WM_ERASEBKGND => 1,

		WM_TIMER =>
		{
			on_background_tick();

			0
		}

		WM_CTLCOLOREDIT => on_ctl_color( wparam as HDC, TEXT, FIELD, app( |app| app.field.get() ) ),

		// Buttons fill themselves with this before WM_DRAWITEM: the default system brush flashes white
		WM_CTLCOLORBTN => ( unsafe { GetStockObject( NULL_BRUSH ) } ) as LRESULT,

		avatar::WM_AVATARS =>
		{
			authors::on_avatars();

			0
		}

		WM_DESTROY =>
		{
			unsafe { PostQuitMessage( 0 ) };

			0
		}

		_ => unsafe { DefWindowProcW( window, message, wparam, lparam ) },
	}
}

// Fixed size: no resizing, no maximize
fn create_window( class: &[u16] ) -> HWND
{
	let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN;

	let mut rect = RECT { left: 0, top: 0, right: scale( WIDTH ), bottom: scale( HEIGHT ) };

	unsafe { AdjustWindowRectExForDpi( &mut rect, style, 0, 0, app( |app| app.dpi.get() ) as u32 ) };

	let title  = wide( format!( "Hammer++ patcher for INFRA {}", install::VERSION ) );
	let width  = rect.right - rect.left;
	let height = rect.bottom - rect.top;

	unsafe { CreateWindowExW( 0, class.as_ptr(), title.as_ptr(), style, CW_USEDEFAULT, CW_USEDEFAULT, width, height, null_mut(), null_mut(), GetModuleHandleW( null() ), null() ) }
}

fn run_message_loop( window: HWND )
{
	let mut message: MSG = unsafe { std::mem::zeroed() };

	while unsafe { GetMessageW( &mut message, null_mut(), 0, 0 ) } > 0
	{
		if unsafe { IsDialogMessageW( window, &message ) } != 0
		{
			continue;
		}

		unsafe
		{
			TranslateMessage( &message );
			DispatchMessageW( &message );
		}
	}
}

// At the size the metrics give for the DPI
fn load_app_icon( width_metric: i32, height_metric: i32 ) -> HICON
{
	let dpi = app( |app| app.dpi.get() ) as u32;

	unsafe
	{
		let width  = GetSystemMetricsForDpi( width_metric, dpi );
		let height = GetSystemMetricsForDpi( height_metric, dpi );

		LoadImageW( GetModuleHandleW( null() ), APP_ICON, IMAGE_ICON, width, height, LR_DEFAULTCOLOR )
	}
}

fn register_class( class: &[u16] )
{
	let window_class = WNDCLASSEXW
	{
		cbSize:        size_of::<WNDCLASSEXW>() as u32,
		style:         0,
		lpfnWndProc:   Some( window_proc ),
		cbClsExtra:    0,
		cbWndExtra:    0,
		hInstance:     unsafe { GetModuleHandleW( null() ) },
		hIcon:         load_app_icon( SM_CXICON, SM_CYICON ),
		hCursor:       unsafe { LoadCursorW( null_mut(), IDC_ARROW ) },
		hbrBackground: null_mut(),
		lpszMenuName:  null(),
		lpszClassName: class.as_ptr(),
		hIconSm:       load_app_icon( SM_CXSMICON, SM_CYSMICON ),
	};

	unsafe { RegisterClassExW( &window_class ) };
}

fn startup()
{
	unsafe
	{
		SetProcessDpiAwarenessContext( DPI_AWARENESS_CONTEXT_SYSTEM_AWARE );
		CoInitializeEx( null(), ( COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE ) as u32 );
	}

	let dpi   = unsafe { GetDpiForSystem() } as i32;
	let field = unsafe { CreateSolidBrush( FIELD ) };

	app( |app| app.dpi.set( dpi ) );
	app( |app| app.field.set( field ) );

	gdiplus::startup();
}

// GDI+ objects first, GDI+ itself after them
fn shutdown()
{
	avatar::clear();
	gdiplus::shutdown();

	unsafe { CoUninitialize() };
}

pub fn run()
{
	startup();

	let class = wide( CLASS );

	register_class( &class );
	authors::register_class();

	let window = create_window( &class );

	if !window.is_null()
	{
		unsafe { ShowWindow( window, SW_SHOWDEFAULT ) };

		run_message_loop( window );
	}

	shutdown();
}
