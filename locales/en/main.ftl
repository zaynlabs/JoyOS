## Tabs
tab-home = Home
tab-games = Games
tab-apps = Apps
tab-settings = Settings

## Home
greeting-morning = Good morning
greeting-afternoon = Good afternoon
greeting-evening = Good evening
greeting-night = Up late?
home-jump-back = Jump back in
home-your-apps = Your apps
home-empty-title = Nothing to play yet
home-empty-body = Install a game with Steam or Epic and it shows up here by itself.

## Buttons and hints
action-play = Play
action-open = Open
action-back = Back
action-select = Select
action-more = More
action-search = Search
action-menu = Menu
action-tabs = Switch
action-continue = Continue
action-done = Done
action-cancel = Cancel

## Games
games-count = { $count ->
    [one] 1 game
   *[other] { $count } games
}
filter-all = All
filter-steam = Steam
filter-epic = Epic
filter-recent = Played lately
games-search = Find a game
games-none = Nothing matches that.

## Game page
detail-last-played = Last played { $when }
detail-never = Not played yet
detail-pin = Add to Home
detail-unpin = Remove from Home
detail-files = Show files
when-today = today
when-yesterday = yesterday
when-days = { $n ->
    [one] 1 day ago
   *[other] { $n } days ago
}

## Apps
apps-title = Apps
apps-tip = In apps, your controller works as a mouse. Press Start to come back.
apps-files = Files

## Quick menu
quick-title = Quick menu
quick-sound = Sound
quick-brightness = Brightness
quick-mouse = Controller as mouse
quick-return = Back to { $name }
quick-close = Close { $name }
quick-screenshot = Take a screenshot
quick-keyboard = Keyboard
quick-power = Power
on = On
off = Off

## Power
power-title = Power
power-sleep = Sleep
power-restart = Restart
power-off = Turn off
power-desktop = Go to desktop

## Settings
set-language = Language
set-look = Look
set-theme = Theme
theme-dark = Dark
theme-light = Light
set-motion = Motion
motion-full = Full
motion-less = Less
set-sounds = Button sounds
set-controller = Controller
set-swap = Swap A and B
set-speed = Mouse speed
set-rumble = Vibration
set-keyboard = On-screen keyboard
keyboard-joy = JoyOS
keyboard-windows = Windows
set-system = System
set-startup = Open JoyOS when the PC starts
set-rescan = Look for new games
set-about = About
about-line = JoyOS { $version }. Made for the couch.

## Controller
pad-connected = { $name } is ready
pad-none = No controller found. The keyboard works too.

## Messages
toast-starting = Starting { $name }
toast-failed = Couldn't open { $name }
toast-shot = Screenshot saved to Pictures
toast-found = { $count ->
    [one] Found 1 game
   *[other] Found { $count } games
}

## Keyboard
key-space = Space
key-delete = Delete
key-shift = Shift
key-enter = Enter
osk-caret = Move

## First run
hello = Hello
setup-language = Pick your language
setup-pad-title = Grab your controller
setup-pad-body = Press A to go on. A keyboard works too.
setup-finding = Finding your games
setup-ready = You're all set
setup-start = Let's go
