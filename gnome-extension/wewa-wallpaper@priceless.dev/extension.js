/**
 * Wewa Wallpaper — GNOME Shell companion extension.
 *
 * Detects wewa windows (by WM_CLASS "wewa-wallpaper") and enforces wallpaper
 * behaviour: pinned below all windows, hidden from Alt+Tab and Activities
 * overview, visible on every workspace.
 *
 * Targets GNOME Shell 45+ (ES module format).
 */

import Meta from 'gi://Meta';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const WM_CLASS = 'wewa-wallpaper';

export default class WewaWallpaperExtension {
    /** @type {Map<Meta.Window, number[]>} managed windows → signal ids */
    _managed = new Map();

    /** @type {number[]} global signal ids */
    _globalSignals = [];

    /** @type {number[]} overview signal ids */
    _overviewSignals = [];

    _origListWindows = null;
    _origGetWindowActors = null;

    enable() {
        // Intercept newly created windows.
        this._globalSignals.push(
            global.display.connect('window-created', (_display, win) => {
                this._tryManage(win);
            }),
        );

        // Re-lower wewa windows after workspace switches.
        this._globalSignals.push(
            global.workspace_manager.connect('active-workspace-changed', () => {
                this._relowerAll();
            }),
        );

        // Patch list_windows to filter wewa from Alt+Tab.
        this._origListWindows = Meta.Workspace.prototype.list_windows;
        const self = this;
        Meta.Workspace.prototype.list_windows = function () {
            const windows = self._origListWindows.call(this);
            return windows.filter(w => !self._managed.has(w));
        };

        // Patch get_window_actors so that overview workspace thumbnails
        // never receive wewa actors when building their clones.
        this._origGetWindowActors = global.get_window_actors.bind(global);
        global.get_window_actors = () => {
            const actors = this._origGetWindowActors();
            if (!Main.overview.visible && !Main.overview.animationInProgress)
                return actors;
            // During overview, exclude managed actors.
            return actors.filter(a => !this._managed.has(a.meta_window));
        };

        // Hide wewa actors during overview so they don't appear as
        // thumbnails.  We listen to `showing` (fires at animation start,
        // before clones are built) and `hidden` (animation finished).
        this._overviewSignals.push(
            Main.overview.connect('showing', () => this._setActorsVisible(false)),
        );
        this._overviewSignals.push(
            Main.overview.connect('hidden', () => {
                this._setActorsVisible(true);
                this._relowerAll();
            }),
        );

        // Manage any wewa windows that already exist.
        for (const actor of this._origGetWindowActors()) {
            this._tryManage(actor.meta_window);
        }
    }

    disable() {
        // Restore patches.
        if (this._origListWindows) {
            Meta.Workspace.prototype.list_windows = this._origListWindows;
            this._origListWindows = null;
        }
        if (this._origGetWindowActors) {
            global.get_window_actors = this._origGetWindowActors;
            this._origGetWindowActors = null;
        }

        // Restore actor visibility.
        this._setActorsVisible(true);

        for (const id of this._overviewSignals)
            Main.overview.disconnect(id);
        this._overviewSignals = [];

        for (const id of this._globalSignals)
            global.display.disconnect(id);
        this._globalSignals = [];

        for (const [win, ids] of this._managed) {
            for (const id of ids) {
                try { win.disconnect(id); } catch (_) { /* already gone */ }
            }
        }
        this._managed.clear();
    }

    // -- window management ---------------------------------------------------

    _tryManage(win) {
        if (this._managed.has(win))
            return;

        if (this._isWewaWindow(win)) {
            this._manageAsWallpaper(win);
            return;
        }

        // WM_CLASS might arrive a tick later.
        const id = win.connect('notify::wm-class', () => {
            if (this._isWewaWindow(win)) {
                win.disconnect(id);
                this._manageAsWallpaper(win);
            }
        });
    }

    _isWewaWindow(win) {
        const cls = win.get_wm_class();
        return cls === WM_CLASS
            || win.get_wm_class_instance?.() === WM_CLASS
            || win.get_title?.() === WM_CLASS;
    }

    _manageAsWallpaper(win) {
        if (this._managed.has(win))
            return;

        const signals = [];

        win.stick();
        win.lower();

        // Re-lower whenever raised.
        signals.push(win.connect('raised', () => win.lower()));

        // Re-lower if it somehow gets focus.
        signals.push(win.connect('notify::appears-focused', () => {
            if (win.appears_focused)
                win.lower();
        }));

        // Prevent accidental minimisation.
        signals.push(win.connect('notify::minimized', () => {
            if (win.minimized)
                win.unminimize();
        }));

        // Cleanup on window destroy.
        signals.push(win.connect('unmanaged', () => {
            const ids = this._managed.get(win);
            if (ids) {
                for (const id of ids) {
                    try { win.disconnect(id); } catch (_) {}
                }
            }
            this._managed.delete(win);
        }));

        // Push actor to bottom of window_group.
        const actor = win.get_compositor_private();
        if (actor)
            global.window_group.set_child_below_sibling(actor, null);

        this._managed.set(win, signals);
    }

    _relowerAll() {
        for (const [win] of this._managed) {
            try {
                win.lower();
                const actor = win.get_compositor_private();
                if (actor)
                    global.window_group.set_child_below_sibling(actor, null);
            } catch (_) {}
        }
    }

    _setActorsVisible(visible) {
        for (const [win] of this._managed) {
            try {
                const actor = win.get_compositor_private();
                if (!actor) continue;
                if (visible)
                    actor.show();
                else
                    actor.hide();
            } catch (_) {}
        }
    }
}
