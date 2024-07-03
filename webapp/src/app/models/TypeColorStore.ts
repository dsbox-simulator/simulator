import { Subject } from 'rxjs';

export class TypeColorStore {
    public static colorMap: { [key: string]: string } = {};
    private static usedColors: Set<string> = new Set();

    static addedNewColor = new Subject<{key: string, color: string}>();

    public static getColor(key: string): string {
        if (!TypeColorStore.colorMap.hasOwnProperty(key)) {
            const newColor = TypeColorStore.generateNewColor();
            TypeColorStore.colorMap[key] = newColor;

            this.addedNewColor.next({ key, color: newColor });
        }

        return TypeColorStore.colorMap[key];
    }

    private static generateNewColor(): string {
        let newColor: string;

        newColor = this.selectColor(Object.keys(this.colorMap).length);

        TypeColorStore.usedColors.add(newColor);
        return newColor;
    }

    private static selectColor(nmb: number): string {
        const hue = nmb * 137.508; // use golden angle approximation
        return `hsl(${hue},100%,50%)`;
    }
}
