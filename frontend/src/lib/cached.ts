export default class Cached<T> {
  value: T | null;
  last: number | null;
  timeout: number;
  fetcher: () => Promise<T>;

  constructor(fetcher: () => Promise<T>, timeout?: number, value?: T) {
    this.value = value ?? null;
    this.timeout = timeout ?? 3600 * 1000;
    this.last = value === undefined ? null : Date.now();
    this.fetcher = fetcher;
  }

  async get() {
    if (
      this.last == null ||
      this.value == null ||
      Date.now() - this.last > this.timeout
    ) {
      this.value = await this.fetcher();
      this.last = Date.now();
    }

    return this.value;
  }
}
