export interface NewsItem {
  id: string;
  title: string;
  desc?: string;
  cover?: string;
  author?: string;
  timestamp?: string;
  hot?: number;
  url: string;
  mobile_url?: string;
}

export interface NewsSource {
  name: string;
  title: string;
  description: string;
  link: string;
  items: NewsItem[];
  total: number;
  from_cache: boolean;
  update_time: string;
  expires_at?: string;
  ttl_minutes?: number;
}

export interface NewsSourceConfig {
  name: string;
  title: string;
  description: string;
  icon: string;
  color: string;
}

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  message?: string;
}
