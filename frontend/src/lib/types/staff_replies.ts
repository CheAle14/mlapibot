export interface StaffReplyThread {
  post_id: string;
  our_comment_id: string;
  created_at: Date;
  suffix: string | null;
}

export interface StaffReply {
  comment_id: string;
  created_at: Date;
  last_updated: Date;
  author_name: string;
  content: string;
}
