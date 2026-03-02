export function removeFirstInArrayBy<T>(
  array: T[],
  predicate: (item: T) => boolean,
): T | undefined {
  const idx = array.findIndex(predicate);

  if (idx !== -1) {
    const [item] = array.splice(idx, 1);
    return item;
  } else {
    return undefined;
  }
}

export function removeAtIndex<T>(array: T[] | undefined, idx: number) {
  if (array) {
    array.splice(idx, 1);
  }
}

export function updateFirstInArrayBy<T>(
  array: T[],
  predicate: (item: T) => boolean,
  update: Partial<T>,
): T | undefined {
  const idx = array.findIndex(predicate);

  if (idx !== -1) {
    const item = array[idx];

    for (const [key, value] of Object.entries(update)) {
      item[key as keyof T] = value as T[keyof T];
    }

    return item;
  } else {
    return undefined;
  }
}

export function updateOrInsertInArrayBy<T>(
  array: T[],
  predicate: (item: T) => boolean,
  replace: T,
  update?: Partial<T>,
) {
  const idx = array.findIndex(predicate);

  if (idx === -1) {
    array.push(replace);
  } else {
    if (update) {
      array[idx] = {
        ...array[idx],
        ...update,
      };
    } else {
      array[idx] = replace;
    }
  }
}
