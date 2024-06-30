import { Subject } from 'rxjs';

export class TypeColorStore {
    public static colorMap: { [key: string]: string } = {};
    private static usedColors: Set<string> = new Set();


    static addedNewColor = new Subject<string>();

    public static getColor(key: string): string {
        if (!TypeColorStore.colorMap.hasOwnProperty(key)) {
            TypeColorStore.colorMap[key] = TypeColorStore.generateNewColor();
        }

        this.addedNewColor.next("");
        return TypeColorStore.colorMap[key];
    }

    private static generateNewColor(): string {
        let newColor: string;

        newColor = this.selectColor(Object.keys(this.colorMap).length)

        TypeColorStore.usedColors.add(newColor);
        return newColor;
    }

    private static selectColor(nmb: number): string {
        const hue = nmb * 137.508; // use golden angle approximation
        return `hsl(${hue},100%,50%)`;
      }

    
}
