"""Finder installer layout, written without Finder automation by dmgbuild."""
from pathlib import Path

application = Path(defines["app"]).resolve()
files = [str(application)]
symlinks = {"Applications": "/Applications"}
format = "UDZO"
filesystem = "HFS+"
volume_name = "Install AppDock"
background = str(Path(defines["background"]).resolve())
icon = str(application / "Contents/Resources/AppDock.icns")
icon_locations = {"AppDock.app": (170, 200), "Applications": (490, 200)}
window_rect = ((180, 140), (660, 400))
default_view = "icon-view"
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
arrange_by = None
icon_size = 96
text_size = 14
# Do not set FinderInfo on the signed app (including hide_extensions).
# Finder recognizes .app bundles without modifying their extended attributes.
