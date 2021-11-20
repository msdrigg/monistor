// @ts-ignore
const Me = imports.misc.extensionUtils.getCurrentExtension();

const { GLib } = imports.gi;
const Main = imports.ui.main;
import * as log from 'log';
import * as monitor_service from 'monitor_service';

class Extension {
    private signals: Map<GObject.Object, Array<SignalID>> = new Map();
    private last_confirm_display_change_time: number = 0;
    // @ts-ignore
    private monitor_service: monitor_service.MonitorService | null = null;

    constructor() {
        // Open monitor config rs to sense and respond to changes
        // Make sure we don't open it before the dbus is available
        GLib.timeout_add(GLib.PRIORITY_DEFAULT, 200, () => {
            this.monitor_service = new monitor_service.MonitorService();
        });
    }

    /// Connects a callback signal to a GObject, and records the signal.
    connect(object: GObject.Object, property: string, callback: (...args: any) => boolean | void): SignalID {
        const signal = object.connect(property, callback);
        const entry = this.signals.get(object);
        if (entry) {
            entry.push(signal);
        } else {
            this.signals.set(object, [signal]);
        }

        return signal;
    }

    signals_remove() {
        for (const [object, signals] of this.signals) {
            for (const signal of signals) {
                object.disconnect(signal);
            }
        }
        this.signals.clear();
    }

    enable() {
        this.signals_attach();
    }
    disable() {
        this.signals_remove();
        this.monitor_service?.exit();
        this.monitor_service = null;
    }

    signals_attach() {
        this.connect(Main.layoutManager, 'system-modal-opened', () => {
            log.log('System modal opened');
            if (Main.modalActorFocusStack.length > 0) {
                let expected_actor = Main.modalActorFocusStack[Main.modalActorFocusStack.length - 1];

                GLib.timeout_add(GLib.PRIORITY_DEFAULT, 200, () => {
                    // Make sure we had a display change in the last 1 seconds
                    if (this.last_confirm_display_change_time > Date.now() - 1000) {
                        log.log('Closing system modal');
                        expected_actor.actor.close();
                    } else {
                        log.log('System modal detected, but it was not shown to be a monitor modal')
                    }
                });
            } else {
                log.warn("Weird event detected, system modal signal emitted but no modals in stack")
            }
        });
        this.connect(global.window_manager, "confirm-display-change", () => {
            this.last_confirm_display_change_time = Date.now();
            log.log(`Confirming monitor change automatically`);
            global.window_manager.complete_display_change(true);
        })
    }
}

// @ts-ignore
function init() {
    return new Extension();
}
