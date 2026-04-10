import axios from 'axios'

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8080'

export function useApi() {
  const get = async (url: string) => {
    const response = await axios.get(`${API_BASE_URL}${url}`)
    return response.data
  }

  const post = async (url: string, data: any) => {
    const response = await axios.post(`${API_BASE_URL}${url}`, data)
    return response.data
  }

  const put = async (url: string, data: any) => {
    const response = await axios.put(`${API_BASE_URL}${url}`, data)
    return response.data
  }

  const del = async (url: string) => {
    const response = await axios.delete(`${API_BASE_URL}${url}`)
    return response.data
  }

  return {
    get,
    post,
    put,
    delete: del
  }
}
