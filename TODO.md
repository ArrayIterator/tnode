# TODO

## CORE

- *DONE* Adding dynamic SSL Keys/Cert on Server Factory (Via Swapping ARC)

## LIBRARIES

### MAXMIND

- Fixing MaxMind library to support `maxminddb` format (currently only supports `mmdb` format).
- Implementing a custom MaxMind reader that can handle both `mmdb` and `maxminddb` formats, ensuring compatibility with a wider range of MaxMind databases.
- Auto download MaxMind database from the official website and update it regularly to ensure the application always has access to the latest geolocation data.

### DOWNLOAD MANAGER

- Implementing a download manager that can handle downloading MaxMind databases from the official website, including support for resuming interrupted downloads and verifying the integrity of downloaded files.
