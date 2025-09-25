from urllib.request import urlopen
import tkinter as tk
from tkinter import ttk, scrolledtext, messagebox
import threading
from datetime import datetime
from bs4 import BeautifulSoup
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.chrome.options import Options
from selenium.common.exceptions import (
    StaleElementReferenceException,
    ElementClickInterceptedException,
)
import time
import json
from pathlib import Path
from typing import List, Dict, Tuple
import re

BASE_URL = "https://eview.eplan.com/"
DEBUG = True


def main():
    # print_from_link("https://www.python.org/")
    # extractor = TestExtractor()
    # extractor.test()

    root = tk.Tk()
    EPlanExtractorGUI(root)
    root.mainloop()


class TestExtractor:
    def __init__(self):
        self.driver = None

    def setup_driver(self):
        options = Options()
        options.add_argument("--no-sandbox")
        options.add_argument("--disable-dev-shm-usage")
        options.add_argument("--disable-gpu")
        options.add_argument("--window-size=1920,1080")

        self.driver = webdriver.Chrome(options=options)
        print("WebDriver successfully started")

    def test(self):
        try:
            self.setup_driver()
            self.driver.get("https://www.python.org/")
            elements = self.driver.find_elements(By.TAG_NAME, "button")
            for element in elements:
                print(element)
        except Exception as e:
            print("An error occurred: ", e)
        finally:
            if self.driver:
                time.sleep(5)
                self.driver.quit()


class EPlanExtractorGUI:
    def __init__(self, root):
        self.root = root
        self.root.title("EPLAN eVIEW SPS-Tabellen Extractor")
        if DEBUG:
            self.root.geometry("900x700")
        else:
            self.root.geometry("480x300")

        self.extractor = None
        self.is_running = False

        self.setup_ui()
        self.load_config()

    def setup_ui(self):
        # Main frame
        main_frame = ttk.Frame(self.root, padding="10")
        main_frame.grid(row=0, column=0, sticky=(tk.W, tk.E, tk.N, tk.S))

        # Configuration area
        config_frame = ttk.LabelFrame(main_frame, text="Configuration", padding="10")
        config_frame.grid(row=0, column=0, columnspan=2, sticky=(tk.W, tk.E), pady=5)

        # Microsoft Email
        ttk.Label(config_frame, text="Microsoft Email:").grid(
            row=1, column=0, sticky=tk.W, padx=5, pady=2
        )
        self.username_var = tk.StringVar()
        self.username_entry = ttk.Entry(
            config_frame, textvariable=self.username_var, width=50
        )
        self.username_entry.grid(row=1, column=1, padx=5, pady=2)

        # Microsoft Password
        ttk.Label(config_frame, text="Microsoft Password:").grid(
            row=2, column=0, sticky=tk.W, padx=5, pady=2
        )
        self.password_var = tk.StringVar()
        self.password_entry = ttk.Entry(
            config_frame, textvariable=self.password_var, width=50, show="*"
        )
        self.password_entry.grid(row=2, column=1, padx=5, pady=2)

        # Projektnummer
        ttk.Label(config_frame, text="Project number:").grid(
            row=3, column=0, sticky=tk.W, padx=5, pady=2
        )
        self.project_var = tk.StringVar()
        self.project_entry = ttk.Entry(
            config_frame, textvariable=self.project_var, width=50
        )
        self.project_entry.grid(row=3, column=1, padx=5, pady=2)

        # Options
        options_frame = ttk.LabelFrame(main_frame, text="Options", padding="10")
        options_frame.grid(row=1, column=0, columnspan=2, sticky=(tk.W, tk.E), pady=5)

        # Checkboxes
        self.headless_var = tk.BooleanVar(value=True)
        if DEBUG:
            ttk.Checkbutton(
                options_frame,
                text="Browser in background (Headless)",
                variable=self.headless_var,
            ).grid(row=0, column=1, sticky=tk.W)

        self.export_excel_var = tk.BooleanVar(value=True)
        ttk.Checkbutton(
            options_frame, text="Export to Excel", variable=self.export_excel_var
        ).grid(row=0, column=0, sticky=tk.W)

        self.export_csv_var = tk.BooleanVar(value=False)
        ttk.Checkbutton(
            options_frame, text="Export to CSV", variable=self.export_csv_var
        ).grid(row=1, column=0, sticky=tk.W)

        # Buttons
        button_frame = ttk.Frame(main_frame)
        button_frame.grid(row=2, column=0, columnspan=2, pady=10)

        self.start_button = ttk.Button(
            button_frame, text="Start extraction", command=self.start_extraction
        )
        self.start_button.grid(row=0, column=0, padx=5)

        self.stop_button = ttk.Button(
            button_frame, text="Stop", command=self.stop_extraction, state="disabled"
        )
        self.stop_button.grid(row=0, column=1, padx=5)

        if DEBUG:
            self.clear_button = ttk.Button(
                button_frame, text="Empty log", command=self.clear_log
            )
            self.clear_button.grid(row=0, column=3, padx=5)

        # Debug Log
        if DEBUG:
            log_frame = ttk.LabelFrame(main_frame, text="Debug Log", padding="10")
            log_frame.grid(
                row=3, column=0, columnspan=2, sticky=(tk.W, tk.E, tk.N, tk.S), pady=5
            )

            self.log_text = scrolledtext.ScrolledText(log_frame, height=20, width=100)
            self.log_text.grid(row=0, column=0, sticky=(tk.W, tk.E, tk.N, tk.S))

        # Statusleiste
        self.status_var = tk.StringVar(value="Ready")
        self.status_bar = ttk.Label(
            main_frame, textvariable=self.status_var, relief=tk.SUNKEN
        )
        self.status_bar.grid(row=4, column=0, columnspan=2, sticky=(tk.W, tk.E), pady=5)

        # Grid-Konfiguration
        main_frame.columnconfigure(0, weight=1)
        main_frame.rowconfigure(3, weight=1)
        if DEBUG:
            log_frame.columnconfigure(0, weight=1)
            log_frame.rowconfigure(0, weight=1)

    def log(self, message, level="INFO"):
        if not DEBUG:
            return

        timestamp = datetime.now().strftime("%H:%M:%S")
        log_message = f"[{timestamp}] [{level}] {message}\n"

        self.log_text.insert(tk.END, log_message)
        self.log_text.see(tk.END)
        self.root.update_idletasks()

    def clear_log(self):
        self.log_text.delete("1.0", tk.END)

    def load_config(self):
        config_file = Path("eplan_config.json")
        if config_file.exists():
            try:
                with open(config_file, "r") as f:
                    config = json.load(f)
                    self.username_var.set(config.get("email", ""))
                    self.password_var.set(config.get("password", ""))
                    self.project_var.set(config.get("project", ""))
                    self.log("Configuration loaded", "INFO")
            except Exception as e:
                self.log(f"Error while loading the configuration: {e}", "ERROR")

    def save_config(self):
        config = {
            "email": self.username_var.get(),
            "password": self.password_var.get(),
            "project": self.project_var.get(),
        }
        try:
            with open("eplan_config.json", "w") as f:
                json.dump(config, f, indent=2)
            self.log("Configuration saved", "INFO")
        except Exception as e:
            self.log(f"Error while saving the configuration: {e}", "ERROR")

    def start_extraction(self):
        if self.is_running:
            messagebox.showwarning("Warning", "Extraction is already running!")
            return

        # Validierung
        if not all(
            [
                self.username_var.get(),
                self.password_var.get(),
                self.project_var.get(),
            ]
        ):
            messagebox.showerror("Error", "Please fill in all fields.")
            return

        # Konfiguration speichern
        self.save_config()

        # UI aktualisieren
        self.start_button.config(state="disabled")
        self.stop_button.config(state="normal")
        self.status_var.set("Extraction is running...")

        # Extraktion in separatem Thread starten
        self.is_running = True
        thread = threading.Thread(target=self.run_extraction)
        thread.daemon = True
        thread.start()

    def stop_extraction(self):
        self.is_running = False
        self.status_var.set("Stopping extraction...")
        self.log("Stopping extraction...", "WARNING")

        if (
            self.extractor
            and hasattr(self.extractor, "driver")
            and self.extractor.driver
        ):
            self.extractor.driver.quit()

        self.start_button.config(state="normal")
        self.stop_button.config(state="disabled")
        self.status_var.set("Extraction stopped")

    def run_extraction(self):
        try:
            self.log("=" * 50, "INFO")
            self.log("Starting EPLAN eVIEW extraction", "INFO")
            self.log(f"Server: {BASE_URL}", "INFO")
            self.log(f"Project: {self.project_var.get()}", "INFO")
            self.log("=" * 50, "INFO")

            # Erstelle Extraktor-Instanz
            self.extractor = SeleniumEPlanExtractor(
                base_url=BASE_URL,
                username=self.username_var.get(),
                password=self.password_var.get(),
                project_number=self.project_var.get(),
                headless=self.headless_var.get(),
                logger=self.log,
            )

            # Führe Extraktion durch
            extracted_data = self.extractor.run_extraction()

            if extracted_data and self.is_running:
                self.log(
                    f"Successfully extracted {len(extracted_data)} tables", "SUCCESS"
                )

                # Exportiere Daten
                if self.export_excel_var.get():
                    filename = f"EPLAN_{self.project_var.get()}_{datetime.now().strftime('%Y%m%d_%H%M%S')}.xlsx"
                    self.extractor.export_to_excel(extracted_data, filename)
                    self.log(f"Excel-Export: {filename}", "SUCCESS")

                if self.export_csv_var.get():
                    prefix = f"EPLAN_{self.project_var.get()}_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
                    self.extractor.export_to_csv(extracted_data, prefix)
                    self.log(f"CSV-Export with prefix: {prefix}", "SUCCESS")

                self.status_var.set(
                    f"Extraction successful - {len(extracted_data)} Tables"
                )
                messagebox.showinfo(
                    "Success",
                    f"{len(extracted_data)} Tables successfully extracted!",
                )
            else:
                self.log("No tables found or extraction cancelled", "WARNING")
                self.status_var.set("No tables found")

        except Exception as e:
            self.log(f"Error during extraction: {str(e)}", "ERROR")
            self.status_var.set("Error during extraction")
            messagebox.showerror("Error", f"Error during extraction:\n{str(e)}")

        finally:
            self.is_running = False
            self.start_button.config(state="normal")
            self.stop_button.config(state="disabled")


def print_from_link(url):
    html = urlopen(url).read()
    soup = BeautifulSoup(html, features="html.parser")

    # kill all script and style elements
    for script in soup(["script", "style"]):
        script.extract()  # rip it out

    # get text
    text = soup.get_text()

    # break into lines and remove leading and trailing space on each
    lines = (line.strip() for line in text.splitlines())
    # break multi-headlines into a line each
    chunks = (phrase.strip() for line in lines for phrase in line.split("  "))
    # drop blank lines
    text = "\n".join(chunk for chunk in chunks if chunk)

    print(text)


class SeleniumEPlanExtractor:
    def __init__(
        self, base_url, username, password, project_number, headless=True, logger=None
    ):
        self.base_url = base_url
        self.username = username
        self.password = password
        self.project_number = project_number
        self.headless = headless
        self.logger = logger or print
        self.driver = None

    def log(self, message, level="INFO"):
        if not DEBUG:
            return

        if callable(self.logger):
            self.logger(message, level)
        else:
            print(f"[{level}] {message}")

    def run_extraction(self):
        self.setup_driver()
        if not self.click_on_login_with_microsoft():
            raise Exception("Failed to click on 'Login with Microsoft' button")
        if not self.login():
            raise Exception("Failed to login")
        if not self.open_project():
            raise Exception("Failed to open project")
        if not self.switch_to_list_view():
            raise Exception("Failed to switch to list view")
        if not self.extract_tables():
            raise Exception("Failed to extract tables")

    def setup_driver(self):
        self.log("Initialise Chrome WebDriver...", "INFO")

        options = Options()
        if self.headless:
            options.add_argument("--headless")
            self.log("Headless-Mode activated", "INFO")

        options.add_argument("--no-sandbox")
        options.add_argument("--disable-dev-shm-usage")
        options.add_argument("--disable-gpu")
        options.add_argument("--window-size=1920,1080")

        # Deaktiviere Bilder für schnelleres Laden
        prefs = {"profile.managed_default_content_settings.images": 2}
        options.add_experimental_option("prefs", prefs)

        try:
            self.driver = webdriver.Chrome(options=options)
            self.log("WebDriver successfully started", "SUCCESS")
        except Exception as e:
            self.log(f"Error while starting the WebDriver: {e}", "ERROR")
            raise

    def click_on_login_with_microsoft(self):
        self.log(f"Navigating to: {self.base_url}", "INFO")
        self.driver.get(self.base_url)

        for attempt in range(15):
            self.log(f"Looking for Microsoft button... [{attempt + 1}/{15}]", "INFO")
            all_buttons = self.driver.find_elements(By.TAG_NAME, "button")
            self.log(f"Found buttons: {len(all_buttons)}", "DEBUG")
            for i, btn in enumerate(all_buttons[:5]):
                if btn.is_displayed():
                    self.log(
                        f"Button {i}: '{btn.text}' | Value: '{btn.get_attribute('value')}' | Class: '{btn.get_attribute('class')}'",
                        "DEBUG",
                    )

            all_elements = self.driver.find_elements(
                By.XPATH,
                "//*[contains(text(), 'Microsoft') or contains(text(), 'microsoft') or contains(@title, 'Microsoft')]",
            )
            for elem in all_elements:
                try:
                    if elem.is_displayed() and elem.is_enabled():
                        elem.click()
                        time.sleep(1)
                        if "login.microsoft" in self.driver.current_url:
                            return True
                except:
                    continue
            time.sleep(1)
        return False

    def login(self):
        try:
            self.log("Waiting for Microsoft email field...", "INFO")

            email_field = None
            email_selectors = [
                "input[type='email']",
                "input[name='loginfmt']",
                "input[id='i0116']",
                "input[id='email']",
                "input[placeholder*='Email']",
                "input[placeholder*='E-Mail']",
                "input[name='username']",
            ]

            # Email field
            for attempt in range(15):
                self.log(f"Waiting for email field... [{attempt+1}/15]", "DEBUG")
                for selector in email_selectors:
                    try:
                        email_field = self.driver.find_element(
                            By.CSS_SELECTOR, selector
                        )
                        if email_field.is_displayed():
                            self.log(
                                f"Email field found with selector: {selector}", "DEBUG"
                            )
                            break
                    except:
                        continue
                if email_field:
                    break
                time.sleep(1)

            if not email_field:
                raise Exception("Email field not found")

            self.log("Type in email...", "INFO")
            try:
                email_field.clear()
                email_field.send_keys(self.username)
            except:
                raise Exception("Unable to type in email")

            # Weiter Button klicken
            self.log("Looking for 'Next' button...", "INFO")
            next_button_selectors = [
                "input[type='submit']",
                "input[id='idSIButton9']",
                "button[type='submit']",
                "input[value='Next']",
                "input[value='Weiter']",
                "button[id='idSIButton9']",
            ]

            next_clicked = False
            for selector in next_button_selectors:
                try:
                    next_button = self.driver.find_element(By.CSS_SELECTOR, selector)
                    if next_button.is_displayed() and next_button.is_enabled():
                        next_button.click()
                        self.log(
                            f"'Next' button clicked with selector: {selector}", "DEBUG"
                        )
                        next_clicked = True
                        break
                except:
                    continue

            if not next_clicked:
                # Alternative: Enter drücken
                from selenium.webdriver.common.keys import Keys

                email_field.send_keys(Keys.RETURN)
                self.log("Submit-button pressed instead of Next-button", "DEBUG")

            # Warte auf Passwort-Seite
            time.sleep(3)

            # Passwort eingeben
            self.log("Looking for password field...", "INFO")
            password_selectors = [
                "input[type='password']",
                "input[name='passwd']",
                "input[id='i0118']",
                "input[id='passwordInput']",
                "input[placeholder*='Password']",
                "input[placeholder*='Passwort']",
            ]

            password_field = None
            for attempt in range(15):
                for selector in password_selectors:
                    try:
                        password_field = self.driver.find_element(
                            By.CSS_SELECTOR, selector
                        )
                        if password_field.is_displayed():
                            self.log(
                                f"Password field found with selector: {selector}",
                                "DEBUG",
                            )
                            break
                    except:
                        continue

                if password_field:
                    break

                time.sleep(1)
                self.log(f"Waiting for password field... [{attempt+1}/15]", "DEBUG")

            if password_field:
                self.log("Inserting password...", "INFO")
                password_field.clear()
                password_field.send_keys(self.password)

                self.log("Looking for 'Sign-In' button", "INFO")
                signin_button_selectors = [
                    "input[type='submit']",
                    "input[id='idSIButton9']",
                    "button[type='submit']",
                    "input[value='Sign in']",
                    "input[value='Anmelden']",
                    "button[id='idSIButton9']",
                ]

                signin_clicked = False
                for selector in signin_button_selectors:
                    try:
                        signin_button = self.driver.find_element(
                            By.CSS_SELECTOR, selector
                        )
                        if signin_button.is_displayed() and signin_button.is_enabled():
                            signin_button.click()
                            self.log(
                                f"'Sign-In' button clicked with selector: {selector}",
                                "DEBUG",
                            )
                            signin_clicked = True
                            break
                    except:
                        continue

                if not signin_clicked:
                    from selenium.webdriver.common.keys import Keys

                    password_field.send_keys(Keys.RETURN)
                    self.log("Submit pressed instead of 'Log-In' click", "DEBUG")

            else:
                self.log(
                    "Password field not found - maybe 'Single Sign-On' active",
                    "WARNING",
                )

            try:
                for attempt in range(15):
                    self.log(
                        f"Trying to click on 'Yes' button... [{attempt + 1}/15]",
                        "DEBUG",
                    )
                    stay_signed_selectors = [
                        "input[id='idSIButton9']",
                        "input[value='Yes']",
                        "input[value='Ja']",
                        "button[id='idSIButton9']",
                    ]

                    for selector in stay_signed_selectors:
                        try:
                            button = self.driver.find_element(By.CSS_SELECTOR, selector)
                            if button.is_displayed() and button.is_enabled():
                                button.click()
                                self.log(
                                    "'Stay logged in' dialogue answered with 'Yes'",
                                    "DEBUG",
                                )
                                break
                        except:
                            time.sleep(1)
                            continue
                    else:
                        time.sleep(1)
                        continue
                    break
            except:
                raise Exception("Couldn't click on 'Stay logged in' button")

            self.log("Waiting for return to EPLAN eVIEW...", "INFO")
            time.sleep(5)

            current_url = self.driver.current_url
            if "login" not in current_url.lower() and (
                self.base_url in current_url or "eview" in current_url.lower()
            ):
                self.log("Microsoft SSO login successful!", "SUCCESS")
                return True
            else:
                self.log(f"Login status unclear. Current URL: {current_url}", "WARNING")
                self.log(f"Page title: {self.driver.title}", "DEBUG")

        except Exception as e:
            self.log(f"Error during Microsoft login: {e}", "ERROR")

            self.log(f"Current URL: {self.driver.current_url}", "DEBUG")

            try:
                self.log(f"Page title: {self.driver.title}", "DEBUG")
            except:
                pass

            return False

    def open_project(self):
        self.log(f"Navigating to project: {self.project_number}", "INFO")

        try:
            self.log("Waiting for project overview...", "INFO")
            time.sleep(3)

            self.log(
                f"Looking for project '{self.project_number}' in the list...", "INFO"
            )

            # Verschiedene Möglichkeiten wie das Projekt angezeigt werden könnte
            project_selectors = [
                f"//td[contains(text(), '{self.project_number}')]",
                f"//span[contains(text(), '{self.project_number}')]",
                f"//div[contains(text(), '{self.project_number}')]",
                f"//a[contains(text(), '{self.project_number}')]",
                f"//tr[contains(., '{self.project_number}')]",
                f"//*[text()='{self.project_number}']",
            ]

            project_element = None

            for xpath in project_selectors:
                try:
                    elements = self.driver.find_elements(By.XPATH, xpath)
                    if elements:
                        project_element = elements[0]
                        # Finde die übergeordnete Zeile (tr)
                        self.log(f"Project found with XPath: {xpath}", "SUCCESS")
                        break
                except:
                    try:
                        # Fallback: nur das Element selbst
                        project_element = self.driver.find_element(By.XPATH, xpath)
                        self.log(
                            f"Project-element found with XPath: {xpath}", "SUCCESS"
                        )
                        break
                    except:
                        continue

            if not project_element:
                # Liste alle Tabellenzeilen für Debugging
                all_rows = self.driver.find_elements(By.TAG_NAME, "tr")
                self.log(f"Found table rows: {len(all_rows)}", "DEBUG")
                for i, row in enumerate(all_rows[:10]):
                    try:
                        row_text = row.text[:100]  # Erste 100 Zeichen
                        self.log(f"Row {i}: {row_text}", "DEBUG")
                    except:
                        pass

                raise Exception(f"Project '{self.project_number}' not found in list")

            # Projekt auswählen (anklicken)
            self.log("Choosing project...", "INFO")

            # Versuche das Projekt anzuklicken
            try:
                # Scrolle zum Projekt nur wenn das Element noch gültig ist
                try:
                    self.driver.execute_script(
                        "arguments[0].scrollIntoView(true);", project_element
                    )
                except:
                    self.log("Couldn't scroll to element, continuing", "DEBUG")

                project_element.click()
                self.log("Project clicked", "DEBUG")
            except:
                pass

            self.log("Looking for 'Open' button...", "INFO")
            all_buttons = self.driver.find_elements(By.TAG_NAME, "button")
            self.log(f"Found buttons after project click: {len(all_buttons)}", "DEBUG")

            open_button = None

            for idx, btn in enumerate(all_buttons):
                try:
                    btn_text = btn.text.strip() if btn.text else ""
                    btn_value = btn.get_attribute("value") or ""

                    if btn_text or btn_value:
                        self.log(
                            f"Button {idx}: Text='{btn_text}' | Value='{btn_value}'",
                            "DEBUG",
                        )

                    if "öffnen" in btn_text.lower() or "open" in btn_text.lower():
                        if btn.is_displayed() and btn.is_enabled():
                            open_button = btn
                            self.log(f"'Open' button found: '{btn_text}'", "SUCCESS")
                            break
                except:
                    continue

            if open_button:
                self.log("Clicking on 'Open' button...", "INFO")
                try:
                    open_button.click()
                    self.log("'Open' button clicked", "SUCCESS")
                except Exception as e:
                    raise Exception("Unable to click on 'Open' button")

                self.log("Waiting for fully loading the project...", "INFO")
                time.sleep(5)

                # Waiting for sidebar
                try:
                    WebDriverWait(self.driver, 10).until(
                        EC.presence_of_element_located(
                            (
                                By.XPATH,
                                "//div[contains(@class, 'tree') or contains(@class, 'sidebar')]",
                            )
                        )
                    )
                    self.log("Project sidebar found", "SUCCESS")
                except:
                    self.log(
                        "Project sidebar not found, still continuing",
                        "WARNING",
                    )

                current_url = self.driver.current_url
                if (
                    self.project_number in current_url
                    or "project" in current_url.lower()
                    or "viewer" in current_url.lower()
                    or "view" in current_url.lower()
                ):
                    self.log(
                        f"Project '{self.project_number}' successfully opened!",
                        "SUCCESS",
                    )

                    time.sleep(2)
                    return True
                else:
                    if current_url != self.base_url:
                        self.log(
                            "Navigated to new page, project probably opened",
                            "SUCCESS",
                        )
                        return True
                    else:
                        self.log(
                            "Project state unclear, still proceeding...", "WARNING"
                        )
                        return True

            else:
                raise Exception("'Open' button not found")

        except Exception as e:
            self.log(f"Error while navigating to the project: {e}", "ERROR")
            return False

    def switch_to_list_view(self):
        time.sleep(1)
        try:
            # Click on button with three dots
            self.log("Looking for buttons that are 'eplan-icon-button'")
            buttons = self.driver.find_elements(By.TAG_NAME, "eplan-icon-button")
            self.log(f"Found {len(buttons)}")
            for i, btn in enumerate(buttons):
                try:
                    if not btn.is_displayed():
                        continue
                    if not "ev-btn-page-more" in btn.get_attribute("data-t"):
                        continue
                    if "fl-pop-up-open" in btn.get_attribute("class"):
                        self.log("Three dots pop-up is already open", "INFO")
                        break
                    try:
                        btn.click()
                        self.log("Clicked button with three dots.", "INFO")
                        break
                    except:
                        raise Exception("Can't click on button with three dots")
                except Exception as e:
                    self.log(
                        f"Can't find button with three dots, called at index {i}\n{e}",
                        "ERROR",
                    )
                    continue

            buttons = self.driver.find_elements(By.TAG_NAME, "eplan-dropdown-item")
            for btn in buttons:
                try:
                    if not btn.is_displayed():
                        continue
                    if not "ev-page-list-view-btn" in btn.get_attribute("data-name"):
                        continue
                    btn.click()
                    self.log("Clicked 'List' Button", "INFO")
                    break
                except:
                    raise Exception("Can't click on 'List' button")
        except Exception as e:
            self.log(f"Error: {e}", "ERROR")
            return False
        return True

    def extract_tables(self):
        while True:
            scroll_container = self.driver.find_element(
                By.CSS_SELECTOR, "cdk-virtual-scroll-viewport"
            )
            last_height = -1
            plc_diagram_pages = set()
            extracted_page_texts = []

            while True:
                visible_items = self.driver.find_elements(
                    By.TAG_NAME, "pv-page-list-item"
                )

                for i in range(len(visible_items)):
                    try:
                        # Re-fetch element on each iteration to avoid stale refs
                        item = self.driver.find_elements(
                            By.TAG_NAME, "pv-page-list-item"
                        )[i]

                        child_divs = item.find_elements(By.TAG_NAME, "div")
                        if any("PLC-Diagram" in div.text for div in child_divs):
                            outer_html = item.get_attribute("outerHTML")
                            if outer_html not in plc_diagram_pages:
                                plc_diagram_pages.add(outer_html)
                                time.sleep(0.5)  # small delay to stabilize
                                item.click()
                                time.sleep(0.5)  # wait for page to update

                                extracted_page_texts.append(
                                    self.extract_current_plc_diagram_page()
                                )

                    except StaleElementReferenceException:
                        self.log("Element went stale, retrying next element", "Warning")
                        continue
                    except ElementClickInterceptedException:
                        self.log("Click intercepted - skipping element", "Warning")
                        continue

                # Scroll down
                self.driver.execute_script(
                    "arguments[0].scrollTop += 400", scroll_container
                )
                time.sleep(0.5)

                new_height = self.driver.execute_script(
                    "return arguments[0].scrollTop", scroll_container
                )
                if new_height == last_height:
                    break  # reached bottom
                last_height = new_height

            break  # exit the outer while loop after done scrolling

        self.log(f"Total pages found: {len(plc_diagram_pages)}", "DEBUG")
        with open("extracted_pages.json", "w") as f:
            json.dump(extracted_page_texts, f, indent=2)
        return True

    def extract_current_plc_diagram_page(self) -> str:
        while True:
            scroll_container = self.driver.find_element(
                By.CSS_SELECTOR, "cdk-virtual-scroll-viewport"
            )
            last_height = -1
            plc_diagram_pages = set()
            extracted_page_texts = []

            while True:
                visible_items = self.driver.find_elements(
                    By.TAG_NAME, "pv-page-list-item"
                )

                for i in range(len(visible_items)):
                    try:
                        # Re-fetch element on each iteration to avoid stale refs
                        item = self.driver.find_elements(
                            By.TAG_NAME, "pv-page-list-item"
                        )[i]

                        child_divs = item.find_elements(By.TAG_NAME, "div")
                        if any("PLC-Diagram" in div.text for div in child_divs):
                            outer_html = item.get_attribute("outerHTML")
                            if outer_html not in plc_diagram_pages:
                                plc_diagram_pages.add(outer_html)

                                # Klicke auf das Element
                                item.click()
                                self.log(
                                    f"Clicked on PLC-Diagram page {len(plc_diagram_pages)}",
                                    "INFO",
                                )

                                # WICHTIG: Warte bis die Seite vollständig geladen ist
                                time.sleep(2)  # Erhöht von 0.5 auf 2 Sekunden

                                # Optional: Warte auf bestimmtes Element
                                try:
                                    WebDriverWait(self.driver, 5).until(
                                        EC.presence_of_element_located(
                                            (By.TAG_NAME, "svg")
                                        )
                                    )
                                    self.log("SVG content loaded", "DEBUG")
                                except:
                                    self.log(
                                        "Timeout waiting for SVG, continuing anyway",
                                        "WARNING",
                                    )

                                # Extrahiere den Inhalt
                                extracted_text = (
                                    self.extract_current_plc_diagram_page_advanced()
                                )

                                if extracted_text:
                                    extracted_page_texts.append(extracted_text)
                                    self.log(
                                        f"Extracted content from page {len(extracted_page_texts)}",
                                        "SUCCESS",
                                    )
                                else:
                                    self.log(
                                        f"No content extracted from page {len(plc_diagram_pages)}",
                                        "WARNING",
                                    )

                    except StaleElementReferenceException:
                        self.log("Element went stale, retrying next element", "WARNING")
                        continue
                    except ElementClickInterceptedException:
                        self.log("Click intercepted - skipping element", "WARNING")
                        continue
                    except Exception as e:
                        self.log(f"Unexpected error: {e}", "ERROR")
                        continue

                # Scroll down
                self.driver.execute_script(
                    "arguments[0].scrollTop += 400", scroll_container
                )
                time.sleep(0.5)

                new_height = self.driver.execute_script(
                    "return arguments[0].scrollTop", scroll_container
                )
                if new_height == last_height:
                    break  # reached bottom
                last_height = new_height

            break  # exit the outer while loop after done scrolling

        self.log(f"Total PLC-Diagram pages found: {len(plc_diagram_pages)}", "INFO")
        self.log(
            f"Successfully extracted content from {len(extracted_page_texts)} pages",
            "INFO",
        )

        # Speichere die Ergebnisse
        if extracted_page_texts:
            with open("extracted_pages.json", "w", encoding="utf-8") as f:
                json.dump(extracted_page_texts, f, indent=2, ensure_ascii=False)
            self.log(f"Results saved to extracted_pages.json", "SUCCESS")

        return extracted_page_texts

    def extract_current_plc_diagram_page_advanced(self) -> str:
        extracted_content = []

        if not extracted_content:
            try:
                page_source = self.driver.page_source

                import re

                text_pattern = r"<text[^>]*>([^<]+)</text>"
                matches = re.findall(text_pattern, page_source)

                tspan_pattern = r"<tspan[^>]*>([^<]+)</tspan>"
                matches.extend(re.findall(tspan_pattern, page_source))

                if matches:
                    self.log(f"Regex found {len(matches)} text matches", "DEBUG")

                    for match in matches:
                        text = match.strip()
                        if text and len(text) > 2:
                            if not any(
                                skip in text for skip in ["Date", "Datum", "ET 200SP"]
                            ):
                                extracted_content.append(text)

            except Exception as e:
                self.log(f"Page source extraction failed: {e}", "ERROR")

        if extracted_content:
            seen = set()
            unique_content = []
            for item in extracted_content:
                if item not in seen:
                    seen.add(item)
                    unique_content.append(item)

            result = " ".join(unique_content)
            self.log(
                f"Successfully extracted {len(unique_content)} unique text elements",
                "SUCCESS",
            )
            self.log("TRYING TO CALL PARSE")
            parsed_data = self.parse_plc_data(str(result))
            result_string = "; ".join(" ".join(d.values()) for d in parsed_data)
            return result_string

        else:
            self.log("No content could be extracted with any method", "ERROR")

            # Debug: Speichere Page Source für manuelle Analyse
            try:
                debug_file = (
                    f"debug_page_source_{datetime.now().strftime('%Y%m%d_%H%M%S')}.html"
                )
                with open(debug_file, "w", encoding="utf-8") as f:
                    f.write(self.driver.page_source)
                self.log(f"Saved page source for debugging: {debug_file}", "DEBUG")
            except:
                pass

            return ""

    def parse_plc_data(input_string: str) -> List[Dict[str, str]]:

        results = []

        # Teile den String in Zeilen auf (berücksichtige verschiedene Zeilenumbrüche)
        lines = input_string.replace("\r\n", "\n").replace("\r", "\n").split("\n")

        address_pattern = r"\b([IQ]W?\d+\.\d+|[IQ]W\d+)\b"
        function_pattern = r"([A-Za-z][A-Za-z\s]+(?:\d+\.)+\d+(?:\s+[A-Z]+)?)"

        current_function = ""

        for line in lines:
            line = line.strip()

            if not line:
                continue

            address_match = re.search(address_pattern, line)

            if address_match:
                address = address_match.group(1)
                text_before_address = line[: address_match.start()].strip()
                function_match = re.search(function_pattern, text_before_address)

                if function_match:
                    current_function = function_match.group(1).strip()
                elif text_before_address and not text_before_address.startswith("="):
                    parts = text_before_address.split()
                    valid_parts = [
                        p
                        for p in parts
                        if not p.startswith("=") and not p.startswith(":")
                    ]
                    if valid_parts:
                        current_function = " ".join(valid_parts)

                if current_function:
                    results.append({"address": address, "function": current_function})

        return results

    def format_plc_table(data: List[Dict[str, str]]) -> str:
        """
        Formatiert die geparsten Daten als übersichtliche Tabelle.

        Args:
            data: Liste der geparsten Daten

        Returns:
            Formatierter String als Tabelle
        """
        if not data:
            return "Keine Daten gefunden"

        # Finde maximale Längen für Formatierung
        max_addr_len = max(len(item["address"]) for item in data)
        max_func_len = max(len(item["function"]) for item in data)

        # Header
        output = []
        output.append("=" * (max_addr_len + max_func_len + 7))
        output.append(f"{'Adresse':<{max_addr_len + 2}} | {'Funktion':<{max_func_len}}")
        output.append("-" * (max_addr_len + max_func_len + 7))

        # Daten
        for item in data:
            output.append(
                f"{item['address']:<{max_addr_len + 2}} | {item['function']:<{max_func_len}}"
            )

        output.append("=" * (max_addr_len + max_func_len + 7))

        return "\n".join(output)

    def clean_plc_extraction(raw_text: str) -> List[Dict[str, str]]:
        """
        Noch einfachere Version: Extrahiert nur die relevanten Zeilen mit Adressen.

        Args:
            raw_text: Roher Text aus der Extraktion

        Returns:
            Liste von Dictionaries mit 'address' und 'name' Keys
        """
        results = []
        lines = raw_text.split("\n")

        # Pattern für Zeilen die wir wollen
        # Format: "Funktionsname Nummer WP/HP Adresse Restlicher-Müll"
        line_pattern = re.compile(
            r"([A-Za-z][A-Za-z\s]*?)\s+"  # Funktionsname
            r"([\d.]+(?:\s+[A-Z]+)?)\s+"  # Nummer + optional WP/HP
            r"([IQ]W?\d+\.\d+|[IQ]W\d+)"  # Adresse
        )

        for line in lines:
            line = line.strip()

            # Skip Header-Zeilen
            if any(
                skip in line
                for skip in [
                    "Sheet",
                    "Editor",
                    "Name",
                    "GmbH",
                    "Job",
                    "Creator",
                    "Version",
                    "Approved",
                    "IO-Test:",
                    "symbol name",
                    "Function text",
                    "Type:",
                    "Placement:",
                    "DT:",
                ]
            ):
                continue

            match = line_pattern.search(line)
            if match:
                func_name = match.group(1).strip()
                number = match.group(2).strip()
                address = match.group(3).strip()

                # Kombiniere Funktionsname und Nummer
                full_name = f"{func_name} {number}"

                results.append({"address": address, "name": full_name})
        return results


if __name__ == "__main__":
    main()
