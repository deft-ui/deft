import * as dialogApi from "native";

export class FileDialog {
    /**
     *
     * @param options {ShowFileDialogOptions}
     * @returns {Promise<string[]>}
     */
    show(options) {
        return new Promise((resolve, reject) => {
            dialogApi.showFileDialog({
                dialogType: options.dialogType,
            }, options.window?.handle, (result, data) => {
                if (result) {
                    resolve(data);
                } else {
                    reject(data);
                }
            })
        })

    }
}

navigator.fileDialog = new FileDialog();