import { NgZone } from "@angular/core";
import { invoke, type InvokeArgs } from "@tauri-apps/api/core";
import { listen, type Event, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * Tauri APIs frequently resolve/emit outside Angular's zone.
 * Wrapping the promise creation / event handler re-enters Angular so change
 * detection reliably runs for async results.
 */

export function wrapPromiseInZone<T>(
    zone: NgZone,
    factory: () => Promise<T>,
): Promise<T> {
    // Create a Zone-aware Promise and resolve/reject it from inside the Angular zone,
    // even if `factory()` returns a native/unpatched Promise.
    return zone.run(() => new Promise<T>((resolve, reject) => {
        try {
            factory().then(
                (value) => zone.run(() => resolve(value)),
                (error) => zone.run(() => reject(error)),
            );
        } catch (error) {
            zone.run(() => reject(error));
        }
    }));
}

export function tauriInvoke<T>(
    zone: NgZone,
    command: string,
    args?: InvokeArgs,
): Promise<T> {
    return wrapPromiseInZone(zone, () => invoke<T>(command, args));
}

export function tauriListen<T>(
    zone: NgZone,
    event: string,
    handler: (event: Event<T>) => void,
): Promise<UnlistenFn> {
    return listen<T>(event, (event) => zone.run(() => handler(event)));
}
