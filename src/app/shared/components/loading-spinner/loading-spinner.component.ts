import { CommonModule } from "@angular/common";
import { Component, Input } from "@angular/core";

@Component({
    selector: "app-loading-spinner",
    standalone: true,
    imports: [CommonModule],
    templateUrl: "./loading-spinner.component.html",
    styleUrl: "./loading-spinner.component.css",
    host: {
        "[style.--spinner-size.px]": "size",
        "[style.--spinner-thickness.px]": "thickness",
        "[style.--spinner-speed.ms]": "speedMs",
    },
})
export class LoadingSpinnerComponent {
    @Input() size = 56;
    @Input() thickness = 5;
    @Input() speedMs = 900;
}
