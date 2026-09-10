// One in-flight read at a time. Stopping invalidates pending results as well as timers.
function createSourceWatch({read, success, failure, schedule = setTimeout, cancel = clearTimeout}) {
  let generation = 0, timer = null, busy = false, watching = false, selected = '';
  async function tick(current) {
    if (current !== generation) return;
    if (busy) {
      timer = schedule(() => tick(current), 100);
      return;
    }
    busy = true;
    try {
      const result = await read(selected);
      if (current === generation) success(result);
    } catch (error) {
      if (current === generation) failure(error);
    } finally {
      busy = false;
      if (current === generation && watching) timer = schedule(() => tick(current), 5000);
    }
  }
  function stop() {
    generation += 1;
    watching = false;
    if (timer !== null) cancel(timer);
    timer = null;
  }
  return {
    start(path, repeat = true) {
      stop(); selected = path; watching = repeat;
      return tick(generation);
    },
    stop
  };
}
