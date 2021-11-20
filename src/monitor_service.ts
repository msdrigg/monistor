//@ts-ignore
const Me = imports.misc.extensionUtils.getCurrentExtension()

import * as log from 'log'

const Gio = imports.gi.Gio
const GLib = imports.gi.GLib;


function logOutput(stdout: any) {
    stdout.read_line_async(GLib.PRIORITY_LOW, null, (stdout: any, res: any) => {
        try {
            let line = stdout.read_line_finish_utf8(res)[0];

            if (line !== null) {
                log.warn(`READ: ${line}`);
                logOutput(stdout);
            }
        } catch (e: any) {
            log.error(e);
        }
    });
}

function spawn_process_helper(): any {

    let proc = Gio.Subprocess.new(
        // The program and command options are passed as a list of arguments
        ['monistord'],

        // The flags control what I/O pipes are opened and how they are directed
        Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE
    );
    if (!proc) {
        log.error("Failed to spawn monistord")
    }
    let stdoutStream = new Gio.DataInputStream({
        base_stream: proc.get_stdout_pipe(),
        close_base_stream: true
    });
    logOutput(stdoutStream);
    return proc
}

export class MonitorService {
    child: any | null = null
    cancellable = new Gio.Cancellable();
    attempts: Array<number> = []

    spawn_monistord() {
        if (this.attempts.length >= 3) {
            let last_attempt = this.attempts[0]
            if (last_attempt > Date.now() - 1000 * 60) {
                log.error("Failed to spawn monistord 3 times in 1 minutes, not attempting again")
                return
            }
        }
        this.attempts.push(Date.now())
        if (this.attempts.length > 3) {
            this.attempts.shift()
        }
        log.log("Trying to spawn monistord")
        try {
            let proc = spawn_process_helper();
            this.child = proc;
            proc.wait_async(this.cancellable, (proc: any, result: any) => {
                try {
                    // Strictly speaking, the only error that can be thrown by this
                    // function is Gio.IOErrorEnum.CANCELLED.
                    proc.wait_finish(result);

                    // The process has completed and you can check the exit status or
                    // ignore it if you just need notification the process completed.
                    if (proc.get_successful()) {
                        log.error('Monistord ended successfully??');
                    } else {
                        log.warn('Monistord failed, retrying in 2 seconds');
                        GLib.timeout_add(GLib.PRIORITY_DEFAULT, 2000, () => {
                            // Make sure we had a display change in the last 1 seconds
                            this.spawn_monistord();
                        });
                    }
                } catch (e: any) {
                    log.error(e);
                }
            });
        } catch (why) {
            log.error(`Failed to spawn monistord: ${why}`)
        }
    }

    constructor() {
        this.spawn_monistord()
    }

    exit() {
        log.log("Exiting monistord")
        this.cancellable.cancel()
        this.child.force_exit()
    }
}
