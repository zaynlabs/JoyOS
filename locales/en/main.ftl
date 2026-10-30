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

## Games
games-count = { $count ->
    [one] 1 game
   *[other] { $count } games
}
filter-all = All
filter-steam = Steam
filter-epic = Epic
filter-gog = GOG
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
apps-tip = In apps, your controller works as a mouse. Press Start to come back.
apps-files = Files

## Quick menu
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

## Watch
tab-watch = Watch
watch-lede = Streaming and video, full screen. Add your own sites below.
watch-add = Add a site
action-remove = Remove
text-type-here = Type here
key-voice = Voice
toast-clip = Saved the last 30 seconds.
quick-clip = Save the last 30 seconds
filter-ubisoft = Ubisoft
filter-xbox = Xbox
filter-roms = Classics

## Settings sections
set-library = Games and apps
set-art = Game pictures
art-on = On
art-off = Off
set-rom-add = Add a folder of games
set-family = Family
family-playing = Who's playing
family-everyone = Everyone
family-add = Add someone
family-set-pin = Set a PIN
family-change-pin = Change PIN
family-limit = Up to { $minutes } min a day
family-no-limit = No daily limit
stats-week = hours this week
stats-year = hours this year
stats-minutes = { $minutes } min
set-connect = Connect
conn-bluetooth = Bluetooth and devices
conn-wifi = Wi-Fi
set-phone = Use a phone as the remote
phone-open = On your phone, open { $address }
phone-pin = Enter the code { $pin }
set-plugins = Add-ons
plugins-open = Open the add-ons folder
set-plan = Plan
plan-current = Your plan: { $plan }
plan-free = Free
plan-plus = JoyOS Plus
plan-business = Business
plan-enter-key = Enter a key
plan-activated = { $plan } is ready. Thank you!
plan-bad-key = That key doesn't work. Check it and try again.
plan-needs-plus = Needs JoyOS Plus
plan-needs-business = Needs the Business plan
kiosk-make = Make JoyOS the whole desktop
kiosk-made = Ready. Read both files first, then run each one as administrator.
kiosk-failed = Couldn't make the scripts.
set-crash = Save error reports on this PC
set-crash-note = They stay here. Nothing is sent anywhere.
sync-folder = Shared folder for settings
sync-now = Copy settings now
sync-pulled = Settings updated from your other PC.
sync-pushed = Settings saved to the shared folder.
sync-same = Settings are already up to date.
sync-failed = Couldn't reach the shared folder.
update-address = Address for new versions
update-available = JoyOS { $version } is out.
update-get = Get { $version }
set-scene = Background
scene-still = Still
scene-aurora = Aurora
set-contrast = Stronger contrast
set-text-large = Larger text
theme-auto = Auto
pad-battery = Controller battery: { $pct }%
pad-charging = Charging
member-added = Added.

## PIN pad
pin-ask = Enter the PIN
pin-set = Choose a PIN
pin-clear = Clear
pin-wrong = That's not the PIN.
pin-saved = PIN saved.
pin-switched = { $name } is playing.

## Per-game settings and time limits
profile-default = Default
profile-needs-plus = Settings for each game need JoyOS Plus.
detail-swap = Swap A and B
detail-power = Power
power-balanced = Balanced
power-performance = Performance
power-saver = Battery saver
toast-time-up = { $name }'s time for today is up.

# Review fixes: remote, recovery, voice, PIN safety
art-bad-key = The cover art key isn't working. Check it in Settings.
key-latin = ABC
phone-forget = Forget paired phones
phone-forgot = Phones will need the new code.
phone-paired = Phone connected.
pin-grown-up = A grown-up's PIN, please
pin-not-saved = Couldn't save the PIN. Try again.
pin-typed = { $count } of 6 numbers
pin-wait = Too many tries. Wait { $seconds } seconds.
recovery-title = Back to Windows
recovery-body = Open the Windows desktop, or see how to make the PC start with Windows again.
recovery-desktop = Open the Windows desktop
recovery-script = Show how to put Windows back
remote-title = Remote
remote-move = Move
remote-press = Press
remote-type = Type
remote-send = Send
remote-lost = Lost the TV. Check the Wi-Fi.
remote-typing = A text box is open. Type below.
remote-lb = Left bumper
remote-rb = Right bumper
remote-view = View
remote-menu = Menu
remote-home = Home
remote-pair-title = Pair your phone
remote-pair-body = Enter the six-digit code shown on your TV.
remote-pair-button = Connect
remote-wrong = That code isn't right.
remote-wait = Too many tries. Wait a minute.
stats-summary = { $week } hours this week, { $year } hours this year
voice-listening = Listening…
voice-unavailable = Voice isn't available. Turn on speech in Windows settings.
web-bad-address = That address doesn't look right.
set-test-versions = Offer test versions too
