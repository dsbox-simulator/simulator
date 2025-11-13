// Debounce function
export default function debounce(this:any, func: (...args: any[]) => any, delay: number) {
    let timeout: number;
    return (...args: any[]) => {
        clearTimeout(timeout);
        timeout = setTimeout(() => {
            func.apply(this, args);
        }, delay);
    };
}