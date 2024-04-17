export interface Product {
    name: string;
    description: string;
    url: string;
    download_link: string;
}

export interface DownloadProgress {
    download_id: number;
    filesize: number;
    transfered: number;
    transfer_rate: number;
    percentage: number;
}