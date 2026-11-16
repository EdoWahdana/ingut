# Settings for dmgbuild. Positions match packaging/macos/background.png.
# dmgbuild executes this file, so paths arrive through -D.

app = defines.get("app")  # noqa: F821
background = defines.get("background")  # noqa: F821
icon = defines.get("icon")  # noqa: F821

files = [app]
symlinks = {"Applications": "/Applications"}
icon_locations = {"Ingut.app": (400, 200), "Applications": (560, 200)}

window_rect = ((120, 160), (680, 420))
default_view = "icon-view"
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
sidebar_width = 0
icon_size = 110
text_size = 13
format = "UDZO"
