import * as _ from "moderndash";

export function mergeArrayPendingChanges<T>(
  key: keyof T,
  original: T[],
  pending: Partial<T>[] | undefined,
  depth?: number,
): T[] {
  if (depth !== undefined && depth < 3) {
    $inspect("mergeArr", original, pending);
  }
  if (pending && pending.length > 0) {
    const arr = original.map((item) => {
      const p = pending.find((other) => other[key] === item[key]);
      return p ? { ...item, ...p } : item;
    });

    for (const pend of pending) {
      if (!pending.some((other) => other[key] === pend[key])) {
        // assume that we've pushed a new item.
        arr.push(pend as T);
      }
    }

    return arr;
  } else {
    return [...original];
  }
}

export function mergeObjectPendingChanges<T extends _.GenericObject>(
  original: T,
  pending: Partial<T> | undefined,
  depth?: number,
): T {
  if (depth === 0) {
    $inspect("mergeObj", original, pending);
  }
  const copy = { ...original };

  if (pending) {
    for (const [key, value] of Object.entries(pending)) {
      const existing = copy[key];
      if (_.isPlainObject(value) && _.isPlainObject(existing)) {
        (copy as _.PlainObject)[key] = mergeObjectPendingChanges(
          existing,
          value,
          (depth ?? 0) + 1,
        );
      } else if (isArray(value) && isArray(existing)) {
        (copy as _.PlainObject)[key] = mergeArrayPendingChanges(
          "id",
          existing,
          value,
          (depth ?? 0) + 1,
        );
      } else {
        (copy as _.PlainObject)[key] = value;
      }
    }
  }

  return copy;
}

function isArray(value: any): value is any[] {
  return (
    typeof value === "object" &&
    "length" in value &&
    typeof value.length === "number"
  );
}
