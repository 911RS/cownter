import { useEffect, useRef, useState } from "react";
import { api, onCoreEvent, onDisplayChanged, onReordered } from "./api";
import type { CoreEvent, DisplayConfig, EntryView } from "./types";

/** Live list of entries, kept up to date from core events. */
export function useEntries(onEvent?: (e: CoreEvent, views: Map<number, EntryView>) => void) {
  return useEntriesState(onEvent)[0];
}

/** Like `useEntries`, plus whether the first load has finished. */
export function useEntriesState(onEvent?: (e: CoreEvent, views: Map<number, EntryView>) => void): [Map<number, EntryView>, boolean] {
  const [loaded, setLoaded] = useState(false);
  const [views, setViews] = useState<Map<number, EntryView>>(new Map());
  const ref = useRef(views);
  const cb = useRef(onEvent);
  cb.current = onEvent;

  useEffect(() => {
    let alive = true;
    const unsubs: Promise<() => void>[] = [];
    const reload = () =>
      api
        .listEntries()
        .then((list) => {
          if (!alive) return;
          const m = new Map(list.map((v) => [v.entry.id, v]));
          ref.current = m;
          setViews(m);
          setLoaded(true);
        })
        .catch(() => setLoaded(true));
    unsubs.push(
      onCoreEvent((e) => {
        if (e.type === "entryUpdated") {
          const m = new Map(ref.current);
          m.set(e.view.entry.id, e.view);
          ref.current = m;
          setViews(m);
        } else if (e.type === "entryRemoved") {
          const m = new Map(ref.current);
          m.delete(e.entryId);
          ref.current = m;
          setViews(m);
        }
        cb.current?.(e, ref.current);
      }),
    );
    unsubs.push(onReordered(() => reload()));
    reload();
    return () => {
      alive = false;
      unsubs.forEach((p) => p.then((u) => u()));
    };
  }, []);

  return [views, loaded];
}

export function useDisplay(): [DisplayConfig | null, (c: DisplayConfig) => void] {
  const [cfg, setCfg] = useState<DisplayConfig | null>(null);
  useEffect(() => {
    api.getDisplay().then(setCfg).catch(() => {});
    const un = onDisplayChanged(setCfg);
    return () => {
      un.then((u) => u());
    };
  }, []);
  return [cfg, setCfg];
}

/** Calls `widget_resize` whenever the element's size changes. */
export function useWidgetSize(el: HTMLElement | null, show: boolean, pad = 0) {
  useEffect(() => {
    if (!el) return;
    const last = { current: "" };
    const send = () => {
      const r = el.getBoundingClientRect();
      const w = Math.ceil(r.width + pad * 2);
      const h = Math.ceil(r.height + pad * 2);
      const key = `${w}x${h}:${show}`;
      if (key === last.current || w < 2 || h < 2) return;
      last.current = key;
      api.widgetResize(w, h, show).catch(() => {});
    };
    const ro = new ResizeObserver(send);
    ro.observe(el);
    send();
    return () => ro.disconnect();
  }, [el, show, pad]);
}

export function useNow(ms: number) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), ms);
    return () => clearInterval(t);
  }, [ms]);
  return now;
}

