export interface User {
  id: string;
  name: string;
  admin: boolean;
}

export interface DbUser extends User {
  cookie: string;
  last_sync: Date;
}
