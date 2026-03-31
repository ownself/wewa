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

const WM_CLASS = 'wewa-wallpaper';

export default class WewaWallpaperExtension {
    /** @type {Map<Meta.Window, number[]>} managed windows → signal ids */
    _managed = new Map();

    /** @type {number[]} global signal ids */
    _globalSignals = [];

    /** @type {number|null} patched Alt+Tab filter id */
    _origGetTabList = null;

    enable() {
        // Intercept newly created windows.
        this._globalSignals.push(
            global.display.connect('window-created', (_display, win) => {
                this._tryManage(win);
            }),
        );

        // Re-lower wewa windows after workspace switches (Mutter may
        // re-stack windows when switching).
        this._globalSignals.push(
            global.workspace_manager.connect('active-workspace-changed', () => {
                this._relowerAll();
            }),
        );

        // Patch Meta.Workspace.list_windows / get_tab_list so that Alt+Tab
        // and the Activities overview never see wewa windows.
        this._patchTabList();

        // Manage any wewa windows that already exist (e.g. extension was
        // re-enabled while wewa was running).
        for (const actor of global.get_window_actors()) {
            this._tryManage(actor.meta_window);
        }
    }

    disable() {
        this._unpatchTabList();

        for (const id of this._globalSignals)
            global.display.disconnect(id);
        this._globalSignals = [];

        // Disconnect per-window signals but do NOT destroy the windows —
        // wewa owns them.
        for (const [win, ids] of this._managed) {
            for (const id of ids) {
                try { win.disconnect(id); } catch (_) { /* already gone */ }
            }
        }
        this._managed.clear();
    }

    // -- window management ---------------------------------------------------

    /**
     * If `win` has the expected WM_CLASS, start managing it. The class may
     * not be available yet at `window-created` time, so we also listen for
     * the property change.
     */
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

        // -- stick to all workspaces --
        win.stick();

        // -- push below everything --
        win.lower();

        // Re-lower whenever the window gets raised (e.g. by focus stealing
        // prevention kicking in).
        signals.push(win.connect('raised', () => win.lower()));

        // If the window somehow receives focus, lower it immediately.
        signals.push(win.connect('notify::appears-focused', () => {
            if (win.appears_focused)
                win.lower();
        }));

        // Prevent minimisation (user might hit Super and accidentally
        // minimise it from the overview).
        signals.push(win.connect('notify::minimized', () => {
            if (win.minimized)
                win.unminimize();
        }));

        // When the window is destroyed, clean up.
        signals.push(win.connect('unmanaged', () => {
            const ids = this._managed.get(win);
            if (ids) {
                for (const id of ids) {
                    try { win.disconnect(id); } catch (_) { /* ok */ }
                }
            }
            this._managed.delete(win);
        }));

        // Move the window actor below all siblings in the window group so it
        // renders behind every other window.
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
            } catch (_) { /* window may be gone */ }
        }
    }

    // -- Alt+Tab / overview filtering ----------------------------------------

    /**
     * Monkey-patch `Meta.Workspace.prototype.list_windows` so that wewa
     * windows are excluded from every consumer (Alt+Tab, overview, …).
     */
    _patchTabList() {
        this._origGetTabList = Meta.Workspace.prototype.list_windows;
        const self = this;

        Meta.Workspace.prototype.list_windows = function () {
            const windows = self._origGetTabList.call(this);
            return windows.filter(w => !self._managed.has(w));
        };
    }

    _unpatchTabList() {
        if (this._origGetTabList) {
            Meta.Workspace.prototype.list_windows = this._origGetTabList;
            this._origGetTabList = null;
        }
    }
}
