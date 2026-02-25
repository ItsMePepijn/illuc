import { CommonModule } from "@angular/common";
import { ChangeDetectionStrategy, Component, Input } from "@angular/core";

@Component({
    selector: "app-codex-gui-throbber",
    standalone: true,
    imports: [CommonModule],
    templateUrl: "./throbber.component.html",
    styleUrl: "./throbber.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ThrobberComponent {
    @Input() size = 12;
    @Input() scaleMax = 1.2;
}
